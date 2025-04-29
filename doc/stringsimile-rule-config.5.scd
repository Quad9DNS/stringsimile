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
- Jaro-Winkler
- IDN Confusables
- Damerau-Levenshtein
- Hamming

Each of the rules has specific values that can be used to configure it:
- Levenshtein
  - maximum_distance - maximum Levenshtein distance to allow to consider this rule matched
- Jaro
  - match_percent_threshold - minimum Jaro match percentage to consider the rule matched (float between 0 and 1)
- Jaro-Winkler
  - match_percent_threshold - minimum Jaro-Winkler match percentage to consider the rule matched (float between 0 and 1)
- IDN Confusables - no additional values (values field may be skipped)
- Damerau-Levenshtein
  - maximum_distance - maximum Damerau Levenshtein distance to allow to consider this rule matched
- Hamming
  - maximum_distance - maximum Hamming distance to allow to consider this rule matched
- Soundex
  - minimum_similarity - minimum Soundex similarity to consider this rule matched (max 4 for normal soundex)
  - soundex_type - type of Soundex to use ("normal" or "refined") - "normal" is limited to 4 maximum similarity
- Metaphone
  - max_code_length - maximum code length for Metaphone to generate (defaults to 4) - can be set to null for unlimited
  - metaphone_type - type of Metaphone to use ("normal" or "double")
- NYSIIS
  - strict - can be set to false to disable strict mode and allow NYSIIS codes over 6 characters long - defaults to true

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
  One of "levenshtein", "jaro", "jaro_winkler", "confusables", "damerau_levenshtein", "hamming", "soundex", "metaphone", "nysiis"
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
