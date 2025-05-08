stringsimile-config(5)

# NAME

stringsimile-config - configuration for *stringsimile*(1)

# CONFIGURATION

By default /etc/stringsimile/stringsimile.yaml file is loaded as configuration,
but it is possible to override it using CLI options.

Configuration file uses YAML format with the following keys:

- input
  Configuration for inputs, which provide objects that need to be matched to
  rules. No inputs are used by default.
	- file_path
	  Path to file that should be used as input.
	- stdin
	  Set to true if stdin should be used as input.
	- kafka
	  If stringsimile is built with kafka support, it can be set up as an
	  input
	    - host
	    Hostname of kafka server
	    - port
	    Port of kafka server (defaults to 9092)
	    - topic
	    Kafka topic to subscribe to for input data
	    - identifier
	    Kafka identifier for input data
	    - pointer
	    Kafka pointer for input data
	    - librdkafka_options
	    Key value pairs of options to pass to librdkafka
- output
  Configuration for inputs, where processed objects with match metadata should
  be output to. No outputs are used by default.
	- file_path
	  Path to file that should be used as output.
	- stdout
	  Set to true if stdout should be used as output.
	- kafka
	  If stringsimile is built with kafka support, it can be set up as an
	  output
	    - host
	    Hostname of kafka server
	    - port
	    Port of kafka server (defaults to 9092)
	    - topic
	    Kafka topic to send output to
	    - identifier
	    Kafka identifier for output data
	    - librdkafka_options
	    Key value pairs of options to pass to librdkafka
- matcher
  Configuration for rules matcher.
	- rule_path
	  Path to directory (or file) that contains JSON (or JSONL) files with
	  rules. Check out *stringsimile-rule-config*(5) for information on
	  rules files. Defaults to /var/lib/stringsimile
	- input_field
	  Field from input object to match agains the rules. Defaults to ".domain_name".
	  This field supports simple syntax for accessing nested objects:
	  e.g. ".domain_name" will look for "domain_name" key in root object.
	  ".value.domain_name" will look for "value" in root object
	  and then for "domain_name" in the "value" object.
	  ".domain_names[0]" will take first value in "domain_names" array.
	  Any failures in matching will just pass through the object unmodified.
	- report_all
	  Set to true to put matching metadata in all objects, regardless of
	  match result. Useful for debugging the rules.
- metrics
  Configuration for metrics exporters. Metrics exporters export metrics in
  Prometheus format.
	- file
	  Export metrics to a file. Configured by default to work with
	  prometheus node-exporter.
	    - file_path
	    Path to the file to export to. Defaults to
	    /var/lib/node-exporter/stringsimile.prom
	    - export_interval_secs
	    How often to export metrics to the given file. Defaults to 15.
	    - mode
	    File mode for the created file. Defaults to 644.
	- stdout
	  Export metrics to stdout. Not enabled by default.
	    - export_interval_secs
	    How often to export metrics to stdout.
	- name_prefix
	  Prefix to add to all metrics names. Defaults to stringsimile_
- process
  Configuration for the stringsimile process.
	- log_level
	  Logging level.
	  Valid levels are: "error", "warn", "info", "debug", "trace"
	- threads
	  Number of threads to use. Defaults to number of available cores.

# EXAMPLES

The following example shows default configuration values:
```
input:
  file_path: null
  stdin: false
  kafka: null
output:
  file_path: null
  stdout: false
  kafka: null
matcher:
  rule_path: "/var/lib/stringsimile"
  input_field: ".domain_name"
  report_all: false
metrics:
  file:
    file_path: /var/lib/node-exporter/stringsimile.prom
    export_interval_secs: 15
log_level: "INFO"
```

This example shows a simple configuration for using stdin/stdout:
```
input:
  stdin: true
output:
  stdout: true
```

Default configuration is also available in /etc/stringsimile/stringsimile.yaml.

# SEE ALSO

*stringsimile*(1) *stringsimile-rule-config*(5)

# AUTHORS

Created by Quad9 (https://quad9.net/).
Implemented by Ensar Sarajčić <dev@ensarsarajcic.com>. For more information and
source, check out https://github.com/Quad9DNS/stringsimile
