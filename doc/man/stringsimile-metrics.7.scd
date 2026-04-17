stringsimile-metrics(7)

# NAME

stringsimile-metrics

Stringimile exposes many metrics that can be exported either to stdout or to a file, in prometheus format. By default metrics are exported to "/var/lib/node-exporter/stringsimile.prom"

# CONFIGURATION

Check out *stringsimile-config*(5), for configuration options. Metrics can be
configured under `metrics` section.

# EXPOSED METRICS

All of the listed metrics have the prefix added before the listed name (default `stringsimile_`),
which can be configured in *stringsimile-config*(5).

*System*
Metrics related to the host system.

- `host_cpu_usage`, with label `name` for each available CPU - CPU usage
percentage
- `host_total_memory` - total memory available in the host
- `host_used_memory` - total memory used on the host (not just by `stringsimile`)
- `host_cpu_count` - number of CPUs available on the host
- `service_used_memory` - memory used by stringsimile
- `service_cpu_usage` - stringsimile CPU usage

*Tasks*
Metrics related to string processing tasks. Each task represents processing for
a single input string (all rule sets are processed in order).

- `service_alive_tasks` - number of currently active processing tasks - number
of input strings being processed in parallel
- `service_pending_tasks` - number of input strings that are waiting to be
processed because the maximum number of workers has been reached
- `service_workers_count` - number of available workers. Can be configured in
*stringsimile-config*(5) (`process.threads`)

*Rules*
Metrics related to string groups, rulesets, rules and processors.

- `string_groupnames` - number of string groups in currently loaded rules
- `rule_sets` - number of rule sets in currently loaded rules
- `rules` - number of individual rules in currently loaded rules
- `rule_matches`, with labels `string_group`, `rule_set` and `rule` containing
names of each respectively - number of matches for the specific rule
- `rule_misses`, with labels `string_group`, `rule_set` and `rule` containing
names of each respectively - number of misses for the specific rule
- `rule_errors`, with labels `string_group`, `rule_set` and `rule` containing
names of each respectively - number of errors for the specific rule
- `exclusion_set_exclusions`, with labels `string_group`, `rule_set`
and `preprocessor_index` containing names of each and the index of the exclusion set
preprocessor- number of input strings excluded by this set

*Inputs and outputs*
Metrics related to inputs and outputs.

- `input_objects_read`, with label `input_type` - number of objects read by this
specific input
- `input_bytes_read`, with label `input_type` - number of bytes read by this
specific input
- `input_errors`, with labels `input_type` and `error_type`
("read" or "parse") - number of errors encountered by this specific input
- `output_objects_written`, with label `output_type` - number of objects written by this
specific output
- `output_bytes_written`, with label `output_type` - number of bytes written by this
specific output
- `output_errors`, with labels `output_type` and `error_type`
("write" or "serialization") - number of errors encountered by this specific output

*Misc*

- `last_reload_signal` - timestamp of the last reload signal (SIGHUP) received by
stringsimile
- `process_uptime_secs` - number of seconds the service has been up
- `process_errors`, with label `type` - number of errors encountered by the
service, not related to inputs, outputs or specific rules

# SEE ALSO

*stringsimile-config*(5)

# AUTHORS

Created by Quad9 (https://quad9.net/).
Implemented by Ensar Sarajčić <dev@ensarsarajcic.com>. For more information and
source, check out https://github.com/Quad9DNS/stringsimile

# REPORTING BUGS

Report bugs to the repository issues board (https://github.com/Quad9DNS/stringsimile/issues) where the development and maintenance is primarily done.
