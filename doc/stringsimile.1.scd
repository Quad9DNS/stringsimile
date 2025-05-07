stringsimile(1)

# NAME

stringsimile - Take a target string in JSON-structured stream (kafka) or file form and compare it against a large set of other strings using rules such as Levenshtein, Jaro, Soundex, IDN Confusables and more.

# SYNOPSIS

_stringsimile_  [OPTIONS]

For more information use *--help* option.

# OPTIONS

*-r, --rules <path>*
	Path to the directory (or file) containing the rules [default: /var/lib/stringsimile]

*-c, --config <path>*
	Path to the configuration file [default: /etc/stringsimile/stringsimile.yaml]

*-v, --verbose...*
	Increase log verbosity. May be repeated for further increase.

*-q, --quiet...*
	Decrease log verbosity. May be repeated for further decrease.

*-t, --threads...*
	Set number of threads to use. Defaults to number of cores.

*--input-file <INPUT_FILE>*
	Optionally set file to read input data from

*--input-from-stdin*
	Set to true to read input data from stdin

*--output-file <OUTPUT_FILE>*
	Optionally set file to write output data to

*--output-to-stdout*
	Set to true to write output data to stdout

*--metrics-file <METRICS_FILE>*
	Optionally set file to write metrics data to

*--metrics-to-stdout*
	Set to true to write metrics data to stdout

*--input-field <INPUT_FIELD>*
	Field to take from input JSON object, to match against rules [default: .domain_name]

*--report-all*
	Set to true to always write match metadata in output, even for failed matches.
	This can be useful for debugging rules.

*-h, --help*
	Print help information for this command.

*-V, --version*
	Print version information for this tool.

# EXAMPLES

Using predefined example rule configurations (*stringsimile-rule-config*(5)) and
a simple config (*stringsimile-config*(5)) with just stdin input and stdout
output:

$ echo '{"domain_name": "wikipedoa.zorkgo.xyz"}' | stringsimile --report-all

{"domain_name":"wikipedoa.zorkgo.xyz","stringsimile":{"groups":[{"rule_sets":{"Example rule set":[{"distance":8,"match":false,"rule_type":"levenshtein","split_position":0,"split_string":"wikipedoa"},{"distance":7,"match":false,"rule_type":"levenshtein","split_position":1,"split_string":"zorkgo"},{"match":false,"match_percent":0.5026455026455027,"rule_type":"jaro","split_position":0,"split_string":"wikipedoa"},{"match":false,"match_percent":0.0,"rule_type":"jaro","split_position":1,"split_string":"zorkgo"}],"Test rule set":[{"distance":8,"match":false,"rule_type":"levenshtein","split_position":0,"split_string":"wikipedoa"},{"distance":6,"match":false,"rule_type":"levenshtein","split_position":1,"split_string":"zorkgo"},{"match":false,"match_percent":0.0,"rule_type":"jaro","split_position":0,"split_string":"wikipedoa"},{"match":false,"match_percent":0.0,"rule_type":"jaro","split_position":1,"split_string":"zorkgo"},{"match":false,"rule_type":"confusables","split_position":0,"split_string":"wikipedoa"},{"match":false,"rule_type":"confusables","split_position":1,"split_string":"zorkgo"},{"distance":8,"match":false,"rule_type":"damerau_levenshtein","split_position":0,"split_string":"wikipedoa"},{"distance":6,"match":false,"rule_type":"damerau_levenshtein","split_position":1,"split_string":"zorkgo"}]},"string_group_name":"Example string group"}]}}

Check out the configuration in /etc/stringsimile/stringsimile.yaml and rules in
/var/lib/stringsimile for examples.

# CONFIGURATION

See *stringsimile-config*(5), *stringsimile-rule-config*(5) for details about
file configurations.

# SEE ALSO

*stringsimile-config*(5) *stringsimile-rule-config*(5)

# AUTHORS

Created by Quad9 (https://quad9.net/).
Implemented by Ensar Sarajčić <dev@ensarsarajcic.com>. For more information and
source, check out https://github.com/Quad9DNS/stringsimile

# REPORTING BUGS

Report bugs to the repository issues board (https://github.com/Quad9DNS/stringsimile/issues) where the development and maintenance is primarily done.
