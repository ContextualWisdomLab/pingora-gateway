# Context Map

`Runtime Configuration` is upstream of `Edge Routing`: it translates YAML/DNS into validated domain values. `Edge Routing` is the core bounded context and owns route uniqueness and precedence. `Pingora Delivery` is an Anti-Corruption Layer downstream of Edge Routing; it may depend on Pingora but the domain may not. `Telemetry` observes the delivery lifecycle through bounded counters and route identifiers.

Consumer products are separate bounded contexts. They supply explicit route configuration or deployment composition but must not push tenant rules, application authorization, static-site fallback semantics or certificate lifecycle into the shared gateway without a separately justified reusable contract.

Certificate issuance/rotation is an external capability. This repository currently consumes upstream TLS trust from the OS and does not terminate downstream TLS.
