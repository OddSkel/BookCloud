#!/usr/bin/env bash

set -u

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)
DATA_DIR="${DATA_DIR:-$ROOT_DIR/data/data_clean/normalized_out}"
TRUNCATE_BEFORE_IMPORT="${TRUNCATE_BEFORE_IMPORT:-1}"
HOST_PSQL_HOST="${HOST_PSQL_HOST:-127.0.0.1}"
DB_INTERNAL_PORT="${DB_INTERNAL_PORT:-5432}"

AUTHOR_CONTAINER="${AUTHOR_CONTAINER:-author-db}"
AUTHOR_PORT="${AUTHOR_PORT:-5431}"
AUTHOR_DB_NAME="${AUTHOR_DB_NAME:-db}"
AUTHOR_DB_USER="${AUTHOR_DB_USER:-user}"
AUTHOR_DB_PASSWORD="${AUTHOR_DB_PASSWORD:-password}"

BOOK_CONTAINER="${BOOK_CONTAINER:-book-db}"
BOOK_PORT="${BOOK_PORT:-5432}"
BOOK_DB_NAME="${BOOK_DB_NAME:-db}"
BOOK_DB_USER="${BOOK_DB_USER:-user}"
BOOK_DB_PASSWORD="${BOOK_DB_PASSWORD:-password}"

GENRE_CONTAINER="${GENRE_CONTAINER:-genre-db}"
GENRE_PORT="${GENRE_PORT:-5433}"
GENRE_DB_NAME="${GENRE_DB_NAME:-db}"
GENRE_DB_USER="${GENRE_DB_USER:-user}"
GENRE_DB_PASSWORD="${GENRE_DB_PASSWORD:-password}"

RATING_CONTAINER="${RATING_CONTAINER:-rating-db}"
RATING_PORT="${RATING_PORT:-5434}"
RATING_DB_NAME="${RATING_DB_NAME:-db}"
RATING_DB_USER="${RATING_DB_USER:-user}"
RATING_DB_PASSWORD="${RATING_DB_PASSWORD:-password}"

SUCCESS_COUNT=0
FAIL_COUNT=0

log() {
    printf '[INFO] %s\n' "$*"
}

warn() {
    printf '[WARN] %s\n' "$*" >&2
}

error() {
    printf '[ERROR] %s\n' "$*" >&2
}

container_running() {
    local container="$1"

    docker inspect --format '{{.State.Running}}' "$container" 2>/dev/null | grep -qx 'true'
}

run_psql_in_container() {
    local container="$1"
    local db_user="$2"
    local db_password="$3"
    local db_name="$4"
    local sql="$5"

    docker exec \
        -e PGPASSWORD="$db_password" \
        "$container" \
        psql \
        -v ON_ERROR_STOP=1 \
        -U "$db_user" \
        -d "$db_name" \
        -p "$DB_INTERNAL_PORT" \
        -c "$sql"
}

pipe_csv_to_container() {
    local container="$1"
    local db_user="$2"
    local db_password="$3"
    local db_name="$4"
    local copy_sql="$5"
    local csv_file="$6"

    docker exec \
        -i \
        -e PGPASSWORD="$db_password" \
        "$container" \
        psql \
        -v ON_ERROR_STOP=1 \
        -U "$db_user" \
        -d "$db_name" \
        -p "$DB_INTERNAL_PORT" \
        -c "$copy_sql" < "$csv_file"
}

run_psql_via_host() {
    local host_port="$1"
    local db_user="$2"
    local db_password="$3"
    local db_name="$4"
    local sql="$5"

    PGPASSWORD="$db_password" \
        psql \
        -v ON_ERROR_STOP=1 \
        -h "$HOST_PSQL_HOST" \
        -p "$host_port" \
        -U "$db_user" \
        -d "$db_name" \
        -c "$sql"
}

pipe_csv_via_host() {
    local host_port="$1"
    local db_user="$2"
    local db_password="$3"
    local db_name="$4"
    local copy_sql="$5"
    local csv_file="$6"

    PGPASSWORD="$db_password" \
        psql \
        -v ON_ERROR_STOP=1 \
        -h "$HOST_PSQL_HOST" \
        -p "$host_port" \
        -U "$db_user" \
        -d "$db_name" \
        -c "$copy_sql" < "$csv_file"
}

import_csv() {
    local label="$1"
    local container="$2"
    local host_port="$3"
    local db_name="$4"
    local db_user="$5"
    local db_password="$6"
    local table_name="$7"
    local table_columns_sql="$8"
    local csv_headers="$9"
    local csv_file="${10}"

    local create_sql
    local truncate_sql=""
    local copy_sql

    create_sql="CREATE TABLE IF NOT EXISTS ${table_name} (${table_columns_sql});"
    if [[ "$TRUNCATE_BEFORE_IMPORT" == "1" ]]; then
        truncate_sql="TRUNCATE TABLE ${table_name};"
    fi
    copy_sql="\\copy ${table_name} (${csv_headers}) FROM STDIN WITH (FORMAT csv, HEADER true)"

    log "Importing ${label} from ${csv_file}"

    if [[ ! -f "$csv_file" ]]; then
        error "${label}: CSV file not found: ${csv_file}"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        return 1
    fi

    if container_running "$container"; then
        if ! run_psql_in_container "$container" "$db_user" "$db_password" "$db_name" "$create_sql"; then
            error "${label}: failed to create table ${table_name} in container ${container}"
            FAIL_COUNT=$((FAIL_COUNT + 1))
            return 1
        fi

        if [[ -n "$truncate_sql" ]] && ! run_psql_in_container "$container" "$db_user" "$db_password" "$db_name" "$truncate_sql"; then
            error "${label}: failed to truncate table ${table_name} in container ${container}"
            FAIL_COUNT=$((FAIL_COUNT + 1))
            return 1
        fi

        if pipe_csv_to_container "$container" "$db_user" "$db_password" "$db_name" "$copy_sql" "$csv_file"; then
            log "${label}: import completed through container ${container}"
            SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
            return 0
        fi

        error "${label}: import failed through container ${container}"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        return 1
    fi

    warn "${label}: container ${container} is not running. Trying host port ${host_port}."

    if ! command -v psql >/dev/null 2>&1; then
        error "${label}: psql is not available on the host and container ${container} is unavailable"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        return 1
    fi

    if ! run_psql_via_host "$host_port" "$db_user" "$db_password" "$db_name" "$create_sql"; then
        error "${label}: failed to create table ${table_name} via host port ${host_port}"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        return 1
    fi

    if [[ -n "$truncate_sql" ]] && ! run_psql_via_host "$host_port" "$db_user" "$db_password" "$db_name" "$truncate_sql"; then
        error "${label}: failed to truncate table ${table_name} via host port ${host_port}"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        return 1
    fi

    if pipe_csv_via_host "$host_port" "$db_user" "$db_password" "$db_name" "$copy_sql" "$csv_file"; then
        log "${label}: import completed through host port ${host_port}"
        SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
        return 0
    fi

    error "${label}: import failed via host port ${host_port}"
    FAIL_COUNT=$((FAIL_COUNT + 1))
    return 1
}

import_genre_csvs() {
    local genre_csv="$DATA_DIR/genre.csv"
    local book_genre_csv="$DATA_DIR/book_genre.csv"
    local genre_create_sql="CREATE TABLE IF NOT EXISTS genre (genre_id BIGINT, name TEXT);"
    local book_genre_create_sql="CREATE TABLE IF NOT EXISTS book_genre (book_isbn BIGINT, genre_id BIGINT);"
    local truncate_sql="TRUNCATE TABLE book_genre, genre RESTART IDENTITY CASCADE;"
    local genre_copy_sql="\\copy genre (genre_id, name) FROM STDIN WITH (FORMAT csv, HEADER true)"
    local book_genre_copy_sql="\\copy book_genre (book_isbn, genre_id) FROM STDIN WITH (FORMAT csv, HEADER true)"

    log "Importing genre tables from ${DATA_DIR}"

    if [[ ! -f "$genre_csv" ]]; then
        error "genre.genre: CSV file not found: ${genre_csv}"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        return 1
    fi

    if [[ ! -f "$book_genre_csv" ]]; then
        error "genre.book_genre: CSV file not found: ${book_genre_csv}"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        return 1
    fi

    if container_running "$GENRE_CONTAINER"; then
        if ! run_psql_in_container "$GENRE_CONTAINER" "$GENRE_DB_USER" "$GENRE_DB_PASSWORD" "$GENRE_DB_NAME" "$genre_create_sql"; then
            error "genre.genre: failed to create table genre in container ${GENRE_CONTAINER}"
            FAIL_COUNT=$((FAIL_COUNT + 1))
            return 1
        fi

        if ! run_psql_in_container "$GENRE_CONTAINER" "$GENRE_DB_USER" "$GENRE_DB_PASSWORD" "$GENRE_DB_NAME" "$book_genre_create_sql"; then
            error "genre.book_genre: failed to create table book_genre in container ${GENRE_CONTAINER}"
            FAIL_COUNT=$((FAIL_COUNT + 1))
            return 1
        fi

        if [[ "$TRUNCATE_BEFORE_IMPORT" == "1" ]] && ! run_psql_in_container "$GENRE_CONTAINER" "$GENRE_DB_USER" "$GENRE_DB_PASSWORD" "$GENRE_DB_NAME" "$truncate_sql"; then
            error "genre: failed to truncate genre and book_genre in container ${GENRE_CONTAINER}"
            FAIL_COUNT=$((FAIL_COUNT + 2))
            return 1
        fi

        log "Importing genre.genre from ${genre_csv}"
        if ! pipe_csv_to_container "$GENRE_CONTAINER" "$GENRE_DB_USER" "$GENRE_DB_PASSWORD" "$GENRE_DB_NAME" "$genre_copy_sql" "$genre_csv"; then
            error "genre.genre: import failed through container ${GENRE_CONTAINER}"
            FAIL_COUNT=$((FAIL_COUNT + 1))
            return 1
        fi
        log "genre.genre: import completed through container ${GENRE_CONTAINER}"
        SUCCESS_COUNT=$((SUCCESS_COUNT + 1))

        log "Importing genre.book_genre from ${book_genre_csv}"
        if ! pipe_csv_to_container "$GENRE_CONTAINER" "$GENRE_DB_USER" "$GENRE_DB_PASSWORD" "$GENRE_DB_NAME" "$book_genre_copy_sql" "$book_genre_csv"; then
            error "genre.book_genre: import failed through container ${GENRE_CONTAINER}"
            FAIL_COUNT=$((FAIL_COUNT + 1))
            return 1
        fi
        log "genre.book_genre: import completed through container ${GENRE_CONTAINER}"
        SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
        return 0
    fi

    warn "genre: container ${GENRE_CONTAINER} is not running. Trying host port ${GENRE_PORT}."

    if ! command -v psql >/dev/null 2>&1; then
        error "genre: psql is not available on the host and container ${GENRE_CONTAINER} is unavailable"
        FAIL_COUNT=$((FAIL_COUNT + 2))
        return 1
    fi

    if ! run_psql_via_host "$GENRE_PORT" "$GENRE_DB_USER" "$GENRE_DB_PASSWORD" "$GENRE_DB_NAME" "$genre_create_sql"; then
        error "genre.genre: failed to create table genre via host port ${GENRE_PORT}"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        return 1
    fi

    if ! run_psql_via_host "$GENRE_PORT" "$GENRE_DB_USER" "$GENRE_DB_PASSWORD" "$GENRE_DB_NAME" "$book_genre_create_sql"; then
        error "genre.book_genre: failed to create table book_genre via host port ${GENRE_PORT}"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        return 1
    fi

    if [[ "$TRUNCATE_BEFORE_IMPORT" == "1" ]] && ! run_psql_via_host "$GENRE_PORT" "$GENRE_DB_USER" "$GENRE_DB_PASSWORD" "$GENRE_DB_NAME" "$truncate_sql"; then
        error "genre: failed to truncate genre and book_genre via host port ${GENRE_PORT}"
        FAIL_COUNT=$((FAIL_COUNT + 2))
        return 1
    fi

    log "Importing genre.genre from ${genre_csv}"
    if ! pipe_csv_via_host "$GENRE_PORT" "$GENRE_DB_USER" "$GENRE_DB_PASSWORD" "$GENRE_DB_NAME" "$genre_copy_sql" "$genre_csv"; then
        error "genre.genre: import failed via host port ${GENRE_PORT}"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        return 1
    fi
    log "genre.genre: import completed through host port ${GENRE_PORT}"
    SUCCESS_COUNT=$((SUCCESS_COUNT + 1))

    log "Importing genre.book_genre from ${book_genre_csv}"
    if ! pipe_csv_via_host "$GENRE_PORT" "$GENRE_DB_USER" "$GENRE_DB_PASSWORD" "$GENRE_DB_NAME" "$book_genre_copy_sql" "$book_genre_csv"; then
        error "genre.book_genre: import failed via host port ${GENRE_PORT}"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        return 1
    fi
    log "genre.book_genre: import completed through host port ${GENRE_PORT}"
    SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
}

main() {
    log "CSV directory: ${DATA_DIR}"
    log "Imports continue even if one container or file fails."

    import_csv \
        "author.author" \
        "$AUTHOR_CONTAINER" \
        "$AUTHOR_PORT" \
        "$AUTHOR_DB_NAME" \
        "$AUTHOR_DB_USER" \
        "$AUTHOR_DB_PASSWORD" \
        "author" \
        "author_id BIGINT, name TEXT" \
        "author_id, name" \
        "$DATA_DIR/author.csv"

    import_csv \
        "author.book_author" \
        "$AUTHOR_CONTAINER" \
        "$AUTHOR_PORT" \
        "$AUTHOR_DB_NAME" \
        "$AUTHOR_DB_USER" \
        "$AUTHOR_DB_PASSWORD" \
        "book_author" \
        "book_isbn BIGINT, author_id BIGINT" \
        "book_isbn, author_id" \
        "$DATA_DIR/book_author.csv"

    import_csv \
        "book.book" \
        "$BOOK_CONTAINER" \
        "$BOOK_PORT" \
        "$BOOK_DB_NAME" \
        "$BOOK_DB_USER" \
        "$BOOK_DB_PASSWORD" \
        "book" \
        "isbn BIGINT, name TEXT, url TEXT, summary_clean TEXT, pub_year INTEGER" \
        "isbn, name, url, summary_clean, pub_year" \
        "$DATA_DIR/book.csv"

    import_genre_csvs

    import_csv \
        "rating.rating" \
        "$RATING_CONTAINER" \
        "$RATING_PORT" \
        "$RATING_DB_NAME" \
        "$RATING_DB_USER" \
        "$RATING_DB_PASSWORD" \
        "rating" \
        "book_isbn BIGINT, star_rating DOUBLE PRECISION, num_ratings BIGINT" \
        "book_isbn, star_rating, num_ratings" \
        "$DATA_DIR/rating.csv"

    printf '\n'
    log "Finished. Successes: ${SUCCESS_COUNT} | Failures: ${FAIL_COUNT}"
    if [[ "$FAIL_COUNT" -gt 0 ]]; then
        warn "At least one import failed. Check the error messages above."
    fi
}

main "$@"
