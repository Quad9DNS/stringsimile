# Stringsimile rules configuration

Rule configuration can be put in /var/lib/stringsimile by default, in
multiple files. This can be overridden using CLI options. Files can have
either .json or .jsonl extension. JSON files are expected to have an
array at their root, while JSONL files are expected to have lines of
JSON object.

Each of the JSON objects represents a single string group, which can be
useful to group together related rule sets, that might refer to strings
that are somehow related.

Each rule set can define a single string to match against, high level
options for pre-processing the input string before matching and a list
of rules to try matching against.

Currently the following rules are supported:

- [**Levenshtein**](https://en.wikipedia.org/wiki/Levenshtein_distance)
  Applies the Levenshtein algorithm to find the distance between
  incoming and the target string and considers it matched if the
  distance is lower than the maximum distance provided.

- **Levenshtein substring variant** Uses Levenshtein algorithm, but can
  match on any substring in the incoming string. Since this has to run
  Levenshtein multiple times, it is considerably slower than Levenshtein
  when strings become longer than the target string.

- [**Jaro**](https://en.wikipedia.org/wiki/Jaro%E2%80%93Winkler_distance#Jaro_similarity)
  Uses Jaro similarity to calculate match percentage between incoming
  and the target string and considers the rule matched if the match
  percentage is higher than the threshold provided.

- [**Jaro-Winkler**](https://en.wikipedia.org/wiki/Jaro%E2%80%93Winkler_distance) Uses
  Jaro-Winkler similarity to calculate match percentage between incoming
  and the target string and considers the rule matched if the match
  percentage is higher than the threshold provided.

- [**IDN Confusables**](https://util.unicode.org/UnicodeJsps/confusables.jsp)
  Detects Unicode confusable characters in the incoming string when
  compared to the target string and considers a match if the 2 can be
  confused.

- [**Damerau-Levenshtein**](https://en.wikipedia.org/wiki/Damerau%E2%80%93Levenshtein_distance)
  Applies the Damerau-Levenshtein algorithm to find the distance between
  incoming and the target string and considers it matched if the
  distance is lower than the maximum distance provided.

- **Damerau-Levenshtein substring variant** Uses Damerau-Levenshtein
  algorithm, but can match on any substring in the incoming string.
  Since this has to run Damerau-Levenshtein multiple times, it is
  considerably slower than Damerau-Levenshtein when strings become
  longer than the target string.

- [**Hamming**](https://en.wikipedia.org/wiki/Hamming_distance) Uses Hamming
  distance to compare the incoming and the target string and considers
  it matched if the distance is lower than the maximum distance
  provided.

- [**Soundex**](https://en.wikipedia.org/wiki/Soundex) Applies the Soundex
  phonetic algorithm to detect similar sounding strings. Can be
  configured with minimum similarity score to consider strings a match.

- [**Metaphone**](https://en.wikipedia.org/wiki/Metaphone) Applies the
  Metaphone phonetic algorithm to detect similar sounding strings. Can
  be configured to use the double Metaphone variant.

- [**NYSIIS**](https://en.wikipedia.org/wiki/Match_rating_approach) Applies
  the NYSIIS phonetic algorithm to detect similar sounding strings. Can
  be configured to disable strict mode, allowing the algorithm to
  generate longer codes, affecting match accuracy.

- [**Match Rating**](https://en.wikipedia.org/wiki/Match_rating_approach)
  Applies the Match Rating Approach algorithm to detect similar sounding
  strings.

- [**Bitflip**](https://www.bitfl1p.com/) Checks for possible single char
  bitflips in a string. Can be configured with different valid sets of
  characters and case sensitivity.

- **Regex** Matches input strings against the provided regex pattern. This
  rule ignores the target string of the ruleset and relies exclusively
  on the configured regex pattern.

- **CIDR** Matches input strings against the provided IP address (or
  network) in CIDR notation. This rule ignores the target string of the
  ruleset and relies exclusively on the configured IP.

Each of the rules has specific values that can be used to configure it:

- **Levenshtein**
  - **maximum_distance** - maximum Levenshtein distance to allow
  to consider this rule matched

- Levenshtein substring variant (allows matching on any substring of
  matching length)
  - **maximum_distance** - maximum Levenshtein distance to
  allow to consider this rule matched

- **Jaro**
  - **match_percent_threshold** - minimum Jaro match percentage to
  consider the rule matched (float between 0 and 1)

- **Jaro-Winkler**
  - **match_percent_threshold** - minimum Jaro-Winkler match
  percentage to consider the rule matched (float between 0 and 1)

- **IDN Confusables** - no additional values (values field may be skipped)

- **Damerau-Levenshtein**
  - **maximum_distance** - maximum Damerau Levenshtein
  distance to allow to consider this rule matched

- Damerau-Levenshtein substring variant (allows matching on any
  substring of matching length)
  - **maximum_distance** - maximum Damerau
  Levenshtein distance to allow to consider this rule matched

- **Hamming**
  - **maximum_distance** - maximum Hamming distance to allow to
  consider this rule matched

- **Soundex**
  - **minimum_similarity** - minimum Soundex similarity to consider
  this rule matched (max 4 for normal soundex)
  - **soundex_type** - type of
  Soundex to use ("normal" or "refined") - "normal" is limited to
  4 maximum similarity

- **Metaphone**
  - **max_code_length** - maximum code length for Metaphone to
  generate (defaults to 4) - can be set to null for unlimited
  - **metaphone_type** - type of Metaphone to use ("normal" or "double")

- **NYSIIS**
  - **strict** - can be set to false to disable strict mode and allow
  NYSIIS codes over 6 characters long - defaults to true

- **Match Rating** - no additional values (values field may be skipped)

- **Bitflip** - optional values (values field may be skipped - defaults to
  "dns" char subset with case sensitive matching)
  - **case_sensitive** - can be set to false to make comparison case insensitive
  - **char_subset** - char subset of valid characters to use for bitlips
  ("dns", "printable", "custom")
  - **custom_char_subset** - if "custom" char subset is configured, this represents a string of valid characters to consider for bitflips

- **Regex**
  - **pattern** - regex pattern to match against. This rule ignores
  the `string_match` of the rule set and just uses this pattern.

- **CIDR**
  - **address** - CIDR notation IP address to match against. This rule
  ignores the `string_match` of the rule set and just uses this
  address.

Single string group object has the following keys:

- **name** Name of the string group. Useful to see what group matched in the
  metadata.

- **rule_sets** List of rule sets to match against.

Each rule set has the following keys:

- **name** Name of the rule set. Useful to see which rule set matched in the
  metadata.

- **string_match** Target string to match all input strings against. This
  value will also appear in metadata to make it easy to find what
  matched.

- **preprocessors** List of preprocessors to apply to the input strings,
  before passing them to rules.

- **match_rules** List of rules to match against.

Each rule has the following keys:

- **rule_type** One of "levenshtein", "levenshtein_substring", "jaro",
  "jaro_winkler", "confusables", "damerau_levenshtein",
  "damerau_levenshtein_substring", "hamming", "soundex",
  "metaphone", "nysiis", "match_rating", "bitflip", "regex",
  "cidr"

- **exit_on_match** If set to true and this rule matches the input string,
  this rule set will stop processing and no rules after this one will be
  checked.

- **values** Object dependent on the rule_type used. Some rules don\'t have
  additional values and this field can be skipped for them.

Besides rules, it is possible to define a list of preprocessors in
"preprocessors" array inside a ruleset. The following preprocessors
are supported:

- **Split target** Splits the input string by dot characters, optionally
  ignoring the last part (if `ignore_tld` is set to true). This is
  useful for domain names. Each part will then be passed through the
  rules individually and will get additional metadata if matched, to
  tell which part exactly matched ("split_string" and
  "split_position" keys).

- **Exclusion set** Excludes known input strings from being matched, to
  avoid matching on them unnecessarily. Can be defined as a list of
  regex patterns or exact strings.

- **Punycode** Encodes or decodes input string to and from punycode,
  optionally passing both the original and encoded/decoded string to the
  rules. Each of the value will get additional metadata to tell if it is
  punycoded ("punycode" boolean key).

Each preprocessor has the following keys:

- **preprocessor_type** One of "split_target", "exclusion_set",
  "punycode"

Depending on the preprocessor_type, other keys may be used too:

- **ignore_tld** \[optional - used only for split_target\] If this flag is
  set, TLD part of the domain name will be ignored for matching.

- **regex** \[optional - used only for exclusion_set\] If this flag is set,
  the provided strings will be interpreted as regex patterns.

- **exclusion_set_source** \[used only for exclusion_set\] Source of strings
  for exclusion set. One of "list" or "file". List takes in a list
  of string values in the rules file itself, while file takes a path to
  an external file with a list of strings, one string per line.

- **list** \[used only for exclusion_set, exclusion_set_source=list\] List
  of strings to use for exclusion set.

- **path** \[used only for exclusion_set, exclusion_set_source=file\] Path
  to the file containing strings for the exclusion set.

- **encode** \[optional - used only for punycode\] Whether this punycode
  preprocessor should encode non-ascii strings to punycode. Defaults to
  true.

- **decode** \[optional - used only for punycode\] Whether this punycode
  preprocessor should decode punycode strings. Defaults to true.

- **keep_both** \[optional - used only for punycode\] Whether this punycode
  preprocessor should keep both the original and the encoded/decoded
  string. Defaults to false.

# EXAMPLES

Examples can be found in [/var/lib/stringsimile directory](../distribution/rules).

# RULE EXAMPLES

- **Levenshtein** configured with `{ "values": {
  "maximum_distance": 2 } }` and `"string_match": "test"`, will
  give the following results for:
  1. Input string = tset  
     Result = `{"match": true, "distance": 2}`
  2. Input string = tsettest  
     Result = `{"match": false, "distance": 4}`

- **Levenshtein substring variant** with
  `{ "values": { "maximum_distance": 1 } }` and
  `"string_match": "test"`, will give the following results
  for:
  1. Input string = mytestx  
     Result = `{"match": true, "distance": 0}`
  2. Input string = mytartx  
     Result = `{"match": false, "distance": 2}`

- **Jaro** configured with `{ "values": {
  "match_percent_threshold": 0.9 } }` and `"string_match":
  "test"`, will give the following results for:
  1. Input string = test  
     Result = `{"match": true, "similarity": 1.0}`
  2. Input string = tset  
     Result = `{"match": false, "similarity": 0.83}`

- **Jaro-Winkler** configured with `{ "values": {
  "match_percent_threshold": 0.9 } }` and `"string_match":
  "test"`, will give the following results for:
  1. Input string = test  
     Result = `{"match": true, "similarity": 1.0}`
  2. Input string = tesx  
     Result = `{"match": true, "similarity": 0.93}`

- **IDN Confusables** configured with no additional values and
  `"string_match": "test"`, will give the following results
  for:
  1. Input string = teѕt (with Cyrillic 'ѕ')  
     Result = `{"match": true}`
  2. Input string = toast  
     Result = `{"match": false}`

- **Damerau-Levenshtein** configured with `{
  "values": { "maximum_distance": 1 } }` and `"string_match":
  "test"`, will give the following results for:
  1. Input string = tset  
     Result = `{"match": true, "distance": 1}` (transposition)
  2. Input string = tent  
     Result = `{"match": true, "distance": 1}`

- **Damerau-Levenshtein substring variant** configured with
  `{ "values": { "maximum_distance": 1 } }` and
  `"string_match": "test"`, will give the following results
  for:
  1. Input string = xtsety  
     Result = `{"match": true, "distance": 1}`
  2. Input string = abc  
     Result = `{"match": false}`

- **Hamming** configured with `{ "values": {
  "maximum_distance": 1 } }` and `"string_match": "test"`, will
  give the following results for:
  1. Input string = tent  
     Result = `{"match": true, "distance": 1}`
  2. Input string = tests  
     Result = `{"match": false}` (different length)

- **Soundex** configured with `{ "values": {
  "minimum_similarity": 4 } }` and `"string_match": "Robert"`,
  will give the following results for:
  1. Input string = Rupert  
     Result = `{"match": true, "similarity": 4}`
  2. Input string = Rubin  
     Result = `{"match": false, "similarity": 2}`

- **Metaphone** configured with `{ "values": {
  "max_code_length": 4 } }` and `"string_match": "Smith"`, will
  give the following results for:
  1. Input string = Smyth  
     Result = `{"match": true}`
  2. Input string = Schmidt  
     Result = `{"match": false}`

- **NYSIIS** configured with default values and `"string_match":
  "Macdonald"`, will give the following results for:
  1. Input string = McDonald  
     Result = `{"match": true}`
  2. Input string = Macdonell  
     Result = `{"match": false}`

- **Match Rating** configured with no additional values and
  `"string_match": "Smith"`, will give the following results
  for:
  1. Input string = Smyth  
     Result = `{"match": true}`
  2. Input string = Johnson  
     Result = `{"match": false}`

- **Bitflip** configured with default values and `"string_match":
  "test"`, will give the following results for:
  1. Input string = uest  
     Result = `{"match": true}`
  2. Input string = best  
     Result = `{"match": false}`

- **Regex** configured with `{ "values": { "pattern":
  "\^test\[0-9\]+\$" } }`, will give the following results for:
  1. Input string = test123  
     Result = `{"match": true}`
  2. Input string = testing  
     Result = `{"match": false}`

- **CIDR** configured with `{ "values": { "address":
  "192.168.0.0/24" } }`, will give the following results for:
  1. Input string = 192.168.0.30  
     Result = `{"match": true}`
  2. Input string = 192.168.1.30  
     Result = `{"match": false}`

# See also

- [Main docs](./README.md)
- [Configuration](./configuration.md)
