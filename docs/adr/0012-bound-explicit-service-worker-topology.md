# ADR 0012: Bound explicit service-worker topology

- Status: Proposed
- Date: 2026-09-10
- Bounded contexts: Runtime Isolation, Admin Config
- Depends on: ADR 0011 and the Pingora 0.9.0 consumer candidate in PR #70

## Problem

The production composition roots build Pingora `ServerConf` through the CWL runtime-policy adapter. Before this decision the adapter did not set `ServerConf::threads`, so Pingora 0.9.0 supplied its framework default of one worker thread for each registered service runtime. A host with many CPUs therefore did not become a multi-worker gateway merely because those CPUs were available.

That made issue #46's configured-worker/NUMA acceptance non-executable against the production topology. It also made a future change to Pingora's framework default capable of changing proxy concurrency without an explicit CWL decision.

## Constraints

The gateway owns transport/runtime capacity, not product authentication, business logic, Keyverse identity, or Wardnet/EgressWeave policy. Proxy worker count must be deterministic from validated Admin Config rather than inferred from host CPU count. Existing unreleased version-1 configurations must preserve their observed one-worker proxy topology when the field is omitted. Configuration is untrusted and must not be able to request an unbounded thread fan-out. A topology knob is not performance evidence: representative Linux/NUMA execution remains required before commercial scaling claims.

Pingora defines `ServerConf::threads` as the global worker count each service receives unless that service supplies an override. It is not a process-wide total. Pingora's `HttpProxy` also consumes `ServerConf::threads` when constructing the sharded graceful-shutdown notifier, so the global value must remain aligned with the proxy service worker topology. Both CWL production processes currently register a proxy service and a Prometheus service. The proxy deliberately follows the validated global value; the low-volume Prometheus listener overrides itself to one worker so increasing proxy capacity does not multiply telemetry workers.

## Alternatives

### Keep Pingora's implicit default

Rejected. It prevents deliberate multi-worker characterization and allows supplier default movement to alter process topology.

### Derive workers from available CPU count

Rejected. Host-sensitive implicit behavior makes exact-head capacity evidence harder to reproduce and can oversubscribe a container whose CPU quota differs from visible host topology.

### Scale every registered service with the proxy

Rejected. The metrics endpoint does not share the proxy data-plane capacity requirement. Letting a high proxy-worker setting multiply Prometheus workers would inflate thread/PID pressure, contaminate NUMA contention evidence, and turn an operator data-plane control into unnecessary observability fan-out.

### Accept any positive proxy worker count

Rejected. A syntactically valid configuration could request an extreme number of threads and exhaust process or node resources before meaningful traffic is served.

### Explicit bounded proxy topology with a fixed metrics override

Selected. `service_threads` is admitted by both generic and characterized pg-erd Admin Config, defaults to one when omitted for compatibility, rejects zero, and rejects values above 256. Both production composition roots pass the validated value into `ServerConf::threads`, preserving the value used by `HttpProxy` for its runtime and shutdown-notification sharding. Each production metrics service explicitly sets its Pingora service-level thread override to one.

The 256 proxy-worker ceiling is deliberately a safety ceiling rather than a recommended deployment size. It covers the 128-core NUMA class that motivated this work while preventing unbounded data-plane thread creation. Raising it requires a later contract decision supported by capacity, shutdown-tail, scheduler and contention evidence. Deployments should normally choose a substantially smaller value from measured CPU quota/topology and workload characteristics.

## Consequences

A many-core host no longer masquerades as multi-worker evidence: the exact configured `service_threads` value must be recorded with performance receipts. Existing configurations that omit the field retain one proxy worker, while the metrics service remains one worker regardless of the proxy setting. New profiling and deployment configs can opt into multi-worker proxy execution without introducing provider or product-domain policy into the gateway.

Operators must still account for all registered services and any service-level overrides when estimating process thread fan-out. For the current two-service composition, `service_threads: 4` represents four configured proxy workers plus one configured metrics worker, not four or five total OS process threads; Pingora and its runtimes may own additional threads. Capacity evidence therefore records the registered service count, proxy worker count, metrics worker count, and derived configured service-worker slots separately. The safety ceiling does not replace container CPU/PID/memory limits or representative NUMA profiling.

The explicit-thread `ServerConf` constructor is crate-private. Public runtime-composition functions revalidate the runtime-capacity values they consume before constructing Pingora configuration, so direct `GatewayConfig` construction or raw `Deserialize` of the pg-erd aggregate cannot bypass the zero/ceiling contract. This duplicates only the activation-boundary check, not product policy or supplier authority.

## Verification

The executable contract must prove all of the following on an exact PR head:

- an explicit positive value is accepted in generic and pg-erd Admin Config;
- omission preserves the one-worker proxy compatibility topology;
- zero and 257 fail closed before listener activation;
- the validated value reaches `ServerConf::threads` in both production composition paths;
- both production composition roots keep the Prometheus service on a one-worker service override;
- capacity evidence is fail-closed against registered-service drift and records proxy/metrics worker slots separately;
- direct programmatic or raw-deserialization construction cannot inject invalid runtime-capacity values through the public composition API;
- existing shutdown, routing, TLS, runtime-isolation, supply-chain and bounded-origin load tests remain GREEN.

Issue #46 remains open after this ADR. It closes only after representative configured-worker traffic and shutdown profiling records the selected proxy worker count, registered service count and overrides, CPU/socket/NUMA topology, high keep-alive pressure, shutdown latency distribution, survivor count, CPU/scheduler behavior and, where available, off-CPU/futex evidence.
