wait_for_keycloak() {
    echo "Waiting for Keycloak to be ready..."
    local attempts=0
    until curl -sf "$KEYCLOAK_URL/realms/master" >/dev/null 2>&1; do
        sleep 5
        (( attempts++ ))
        if (( attempts > 60 )); then
            echo "Erro: Keycloak did not become ready after ~5 minutes." >&2
            exit 1
        fi
    done
    echo "Keycloak is ready."
}

ensure_service_account_roles() {
    local TOKEN="$1"
    local CLIENT_UUID="$2"
    
    echo "Ensuring bookcloud-app service account roles..."
    
    local SA_USER_ID
    SA_USER_ID=$(curl -s "$KEYCLOAK_URL/admin/realms/$REALM/clients/$CLIENT_UUID/service-account-user" \
    -H "Authorization: Bearer $TOKEN" | jq -r '.id')
    
    if [ -z "$SA_USER_ID" ] || [ "$SA_USER_ID" = "null" ]; then
        echo "Erro: could not fetch service account user for bookcloud-app" >&2
        exit 1
    fi
    
    local REALM_MGMT_UUID
    REALM_MGMT_UUID=$(curl -s "$KEYCLOAK_URL/admin/realms/$REALM/clients?clientId=realm-management" \
    -H "Authorization: Bearer $TOKEN" | jq -r '.[0].id')
    
    if [ -z "$REALM_MGMT_UUID" ] || [ "$REALM_MGMT_UUID" = "null" ]; then
        echo "Erro: realm-management client not found." >&2
        exit 1
    fi
    
    local ROLE_NAME ROLE_JSON HTTP_STATUS
    
    for ROLE_NAME in manage-users view-users query-users view-realm; do
        ROLE_JSON=$(curl -s "$KEYCLOAK_URL/admin/realms/$REALM/clients/$REALM_MGMT_UUID/roles/$ROLE_NAME" \
        -H "Authorization: Bearer $TOKEN")
        
        if [ -z "$ROLE_JSON" ] || [ "$(echo "$ROLE_JSON" | jq -r '.id // empty')" = "" ]; then
            echo "Erro: could not fetch realm-management role $ROLE_NAME" >&2
            exit 1
        fi
        
        HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
            -X POST "$KEYCLOAK_URL/admin/realms/$REALM/users/$SA_USER_ID/role-mappings/clients/$REALM_MGMT_UUID" \
            -H "Authorization: Bearer $TOKEN" \
            -H "Content-Type: application/json" \
        -d "[$ROLE_JSON]")
        
        if [ "$HTTP_STATUS" != "204" ] && [ "$HTTP_STATUS" != "409" ]; then
            echo "Erro: failed to assign service account role $ROLE_NAME (HTTP $HTTP_STATUS)" >&2
            exit 1
        fi
        
        echo "✓ service account role: $ROLE_NAME"
    done
}

setup_keycloak_users() {
    wait_for_keycloak
    
    local TOKEN
    TOKEN=$(get_admin_token)
    
    local CLIENT_UUID
    CLIENT_UUID=$(curl -s "$KEYCLOAK_URL/admin/realms/$REALM/clients?clientId=bookcloud-app" \
    -H "Authorization: Bearer $TOKEN" | jq -r '.[0].id')
    if [ -z "$CLIENT_UUID" ] || [ "$CLIENT_UUID" = "null" ]; then
        echo "Erro: bookcloud-app client not found in realm '$REALM'." >&2
        exit 1
    fi
    
    local CLIENT_SECRET="bookcloud-app-secret"
    
    curl -s -X PUT "$KEYCLOAK_URL/admin/realms/$REALM/clients/$CLIENT_UUID" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
      \"clientId\": \"bookcloud-app\",
      \"secret\": \"$CLIENT_SECRET\",
      \"enabled\": true,
      \"publicClient\": false,
      \"serviceAccountsEnabled\": true,
      \"directAccessGrantsEnabled\": true,
      \"standardFlowEnabled\": true,
      \"protocol\": \"openid-connect\"
    }" >/dev/null
    
    echo "✓ bookcloud-app client secret fixed: $CLIENT_SECRET"
    
    ensure_service_account_roles "$TOKEN" "$CLIENT_UUID"
    
    create_user "testuser"     "user"
    create_user "adminuser"    "admin"
    create_user "readonlyuser" "readonly"
    
    echo "✓ All users created."
    echo "$CLIENT_SECRET"
}

patch_kong_declarative_config() {
    echo "Fetching Keycloak RS256 signing key from JWKS..."
    local JWKS
    JWKS=$(curl -s "$KEYCLOAK_URL/realms/$REALM/protocol/openid-connect/certs")
    
    local X5C
    X5C=$(echo "$JWKS" | jq -r '.keys[] | select(.alg=="RS256" and .use=="sig") | .x5c[0]')
    if [ -z "$X5C" ] || [ "$X5C" = "null" ]; then
        echo "Erro: failed to extract RS256 x5c certificate from JWKS." >&2
        exit 1
    fi
    
    local PUBKEY_PEM
    PUBKEY_PEM=$(echo "$X5C" \
        | base64 -d \
    | openssl x509 -inform DER -pubkey -noout 2>/dev/null)
    if [ -z "$PUBKEY_PEM" ]; then
        echo "Erro: failed to extract public key from x5c certificate." >&2
        exit 1
    fi
    
    local ISSUER
    # O login é feito pelo api-gateway usando o DNS interno do cluster.
    # Portanto, o claim "iss" dos access tokens é http://keycloak/realms/bookcloud.
    # A key da credential JWT do Kong tem de bater exatamente com esse claim.
    ISSUER="http://keycloak/realms/$REALM"
    local KONG_YML="$SCRIPT_DIR/kong/configmap.yaml"
    
    echo "Applying kong ConfigMap with live public key and issuer..."
    
    # Indent each PEM line by 14 spaces to match kong.yml block scalar indentation
    local INDENT="              "
    local INDENTED_PEM
    INDENTED_PEM=$(echo "$PUBKEY_PEM" | sed "s/^/${INDENT}/")
    
    export AWK_PEM="$INDENTED_PEM"
    export AWK_ISSUER="$ISSUER"
    
    # Replace placeholders in-memory and pipe directly to kubectl — file is never modified
    awk '
    /KONG_JWT_PUBLIC_KEY_PLACEHOLDER/ { print ENVIRON["AWK_PEM"]; next }
    /KONG_JWT_ISSUER_PLACEHOLDER/     { sub(/KONG_JWT_ISSUER_PLACEHOLDER/, ENVIRON["AWK_ISSUER"]) }
    { print }
    ' "$KONG_YML" | kubectl apply -f -
    
    echo "Restarting Kong to load new declarative config..."
    kubectl -n "$NAMESPACE" rollout restart deployment/kong
    kubectl -n "$NAMESPACE" rollout status deployment/kong --timeout="$ROLLOUT_TIMEOUT"
    echo "✓ Kong reloaded with live Keycloak public key."
}