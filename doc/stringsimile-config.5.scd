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
- output
  Configuration for inputs, where processed objects with match metadata should
  be output to. No outputs are used by default.
	- file_path
	  Path to file that should be used as output.
	- stdout
	  Set to true if stdout should be used as output.
- matcher
  Configuration for rules matcher.
	- rule_path
	  Path to directory (or file) that contains JSON (or JSONL) files with
	  rules. Check out *stringsimile-rule-config*(5) for information on
	  rules files. Default to /var/lib/stringsimile
	- input_field
	  Field from input object to match agains the rules. Defaults to ".domain_name".
	  This field supports simple syntax for accessing nested objects:
	  e.g. ".domain_name" will look for "domain_name" key in root object.
	  ".value.domain_name" will look for "value" in root object
	  and then for "domain_name" in the "value" object.
	  Any failures in matching will just pass through the object unmodified.
	- report_all
	  Set to true to put matching metadata in all objects, regardless of
	  match result. Useful for debugging the rules.
- log_level
  Logging level.
  Valid levels are: "error", "warn", "info", "debug", "trace"

# EXAMPLES

The following example shows default configuration values:
```
input:
  file_path: null
  stdin: false
output:
  file_path: null
  stdout: false
matcher:
  rule_path: "/var/lib/stringsimile"
  input_field: ".domain_name"
  report_all: false
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
