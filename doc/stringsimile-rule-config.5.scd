stringsimile-rule-config(5)

# NAME

stringsimile-rule-config - user configuration for *stringsimile*(1)

# CONFIGURATION

Rule configuration can be put in /var/lib/stringsimile by default, in multiple
files. This can be overriden using CLI options. Files can have either .json or .jsonl
extension. JSON files are expected to have an array at their root, while JSONL
files are expected to have lines of JSON object.

Each of the JSON objects represents a single string group, which can be useful
to group together related rule sets, that might refer to strings that are
somehow related.

Each rule set can define a single string to match against, high level options
for pre-processing the input string before matching and a list of rules to try
matching against.

Currently the following rules are supported:
- Levenshtein
- Jaro
- IDN Confusables

Each of the rules has specific values that can be used to configure it:
- Levenshtein
  - maximum_distance - maximum Levenshtein distance to allow to consider this rule matched
- Jaro
  - match_percent_threshold - minimum Jaro match percentage to consider the rule matched
- IDN Confusables - no additional values (values field may be skipped)


Single string group object has the following keys:

- name
  Name of the string group. Useful to see what group matched in the metadata.
- rule_sets
  List of rule sets to match against.

Each rule set has the following keys:
- name
  Name of the rule set. Useful to see which rule set matched in the metadata.
- string_match
  Target string to match all input strings against. This value will also appear
  in metadata to make it easy to find what matched.
- split_target [optional]
  If set to true, input string will be treated as a domain name and will be
  split into domain name parts.
- ignore_tld [optional]
  This field is only valid if split_target is set to true. If this flag is set,
  TLD part of the domain name will be ignored for matching.
- match_rules
  List of rules to match against.

Each rule has the following keys:
- rule_type
  One of "levenshtein", "jaro", "confusables"
- values
  Object dependent on the rule_type used. Some rules don't have additional
  values and this field can be skipped for them.

# EXAMPLES

Examples can be found in /var/lib/stringsimile directory.

# SEE ALSO

*stringsimile*(1) *stringsimile-config*(5)

# AUTHORS

Created by Quad9 (https://quad9.net/).
Implemented by Ensar Sarajčić <dev@ensarsarajcic.com>. For more information and
source, check out https://github.com/Quad9DNS/stringsimile
