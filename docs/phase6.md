# Phase 6 – Non-Functional Requirements and Use cases

## 1. Authentication and Authorization – Keycloak

### Non-Functional Requirement

The system must enforce secure access control across all microservices, ensuring that only authenticated and authorized users can access protected endpoints.

### Use Cases

- A user registers and logs in through a centralized authentication portal managed by Keycloak.
- Different roles (e.g., `admin`, `user`, `readonly`) are assigned to users, granting or restricting access to specific API endpoints.
- A microservice receives a request containing a JWT token and validates it against Keycloak before processing.
- Expired or invalid tokens are rejected with a 401 Unauthorized response.
- An admin user manages roles and permissions through Keycloak's admin interface.

### Implementation Plan

Keycloak will be deployed as a containerized service within the Kubernetes cluster in a dedicated namespace. It will act as the Identity Provider (IdP) for the entire application, exposing an OAuth2/OpenID Connect interface.

Each microservice will be configured as a Keycloak client and will validate incoming Bearer tokens (JWT) against the Keycloak server. The token validation will occur at the API Gateway or directly at each microservice using Keycloak's public key for signature verification.

Role-Based Access Control (RBAC) will be implemented by mapping Keycloak roles to specific API operations. The roles will be embedded in the JWT claims and read at runtime by each service to decide whether to allow or deny a request.

---

## 2. Cache – Redis

### Non-Functional Requirement

The system must reduce latency on frequently accessed endpoints and lower the load on the database by introducing a caching layer.

### Use Cases

- After a user requests a list of items that rarely changes, the result is then served from cache instead of querying the database.
- After a configurable TTL (Time-To-Live), the cached entry expires and the next request triggers a fresh database query, which is then re-cached.
- When data is modified (create/update/delete), the relevant cache entries are invalidated to ensure consistency.
- High-traffic endpoints benefit from near-instant response times by avoiding repeated expensive computations or database queries.

### Implementation Plan

Redis will be deployed as a containerized service within the Kubernetes cluster. The microservices responsible for data retrieval will interact with Redis using the Cache-Aside pattern: the service first checks the cache; if a cache miss occurs, it queries the database and stores the result in Redis with an appropriate TTL.

Cache keys will be designed to reflect the query parameters, ensuring that different request variations are cached independently. TTL values will be tuned per endpoint based on the expected frequency of data changes.

Cache invalidation will be triggered explicitly by write operations (create, update, delete) within the service layer. Redis will be configured with a memory limit and an eviction policy to handle situations where the cache fills up.

---

## 3. DevOps – Prometheus and Terraform

### Non-Functional Requirement

The system must be observable, with metrics collected and visualized in real time, and the cloud infrastructure must be provisioned and managed through code to ensure reproducibility and consistency across environments.

### Use Cases

**Prometheus / Monitoring:**
- An operator monitors the health and performance of all microservices through a Grafana dashboard backed by Prometheus data.
- Alerts are triggered when a microservice's error rate exceeds a threshold or when response times degrade significantly.
- Kubernetes cluster metrics (CPU, memory, pod restarts) are collected and visualized alongside application-level metrics.
- Each microservice exposes a `/metrics` endpoint in the Prometheus exposition format.

**Terraform:**
- The entire cloud infrastructure (GKE cluster, node pools, namespaces, IAM roles, networking) is defined as code in Terraform configuration files.
- A new environment can be provisioned by running Terraform with a different variable set, without manual intervention.
- Infrastructure changes are reviewed and applied through a controlled process, making changes auditable and reversible.

### Implementation Plan

**Prometheus** will be deployed to the Kubernetes cluster and each microservice will be instrumented to expose metrics such as request count, error rate, and response time histograms.

Grafana dashboards will be pre-configured and stored as JSON files in the repository, allowing them to be reproduced in any deployment.

**Terraform** will be used to provision and manage the Google Cloud infrastructure. The configuration will cover the GKE cluster definition, node pool sizing, Kubernetes namespaces, IAM service accounts and bindings, and network configuration.

---


## 4. Database Replication – Fault Tolerance, Load Balancing and Availability

### Non-Functional Requirement

The system must guarantee high database availability and fault tolerance, ensuring that a single node failure does not cause data loss or a complete service outage. Read and write workloads must be distributed across multiple database instances to prevent any single machine from becoming a bottleneck, and the system must recover gracefully from both transient network faults and permanent node failures.

### Use Cases

- The primary database node fails unexpectedly. A replica is automatically promoted to primary, and the microservices resume normal operation with no manual intervention and minimal downtime.
- Under peak traffic, multiple microservices issue a high volume of read queries simultaneously. These queries are spread across several read replicas rather than hitting a single machine, keeping response times stable.
- A replica node becomes unreachable. The load balancer detects this through health checks and stops routing traffic to it. The remaining replicas absorb the load until the failed node recovers or is replaced.
- A write operation to the primary times out. The microservice treats this as a retriable error and re-attempts the operation, while the system ensures idempotency to prevent duplicate writes.

### Implementation Plan

**Replication topology and fault tolerance**

The database will be deployed in a primary-replica replication topology, where a single primary node handles all write operations and one or more replica nodes receive a continuous stream of changes from the primary. This synchronous or semi-synchronous replication ensures that replicas hold an up-to-date copy of the data at all times.

**Relationship between machines and load balancing**

A database-aware load balancer sits between the microservices and the database cluster. It inspects each incoming connection, routing write operations exclusively to the primary and distributing read operations across the available replicas using a round-robin or least-connections strategy. This layer also performs continuous health checks against all nodes and automatically removes unhealthy instances from the routing pool, ensuring that microservices are never directed to an unavailable machine. When a node recovers, it is re-added to the pool automatically once it passes the health checks.

**Timeout at the microservice level**

Every database operation at the microservice data access layer will be wrapped with a configurable connection and query timeout. This prevents requests from blocking indefinitely when a node is slow or unreachable, freeing up application threads and preserving overall service responsiveness.
