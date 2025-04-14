# stringsimile specification

Match target strings from different sources and compare them against a large set of other strings using rules such as Levenshtein, Jaro, Soundex, IDN Confusables and more. While this is built for matching domain names, it should be usable for any kind of string matching.

## General Config

One or more JSON files in a directory will describe the rules. Rules can be added while the process is running and HUP signal can be used to reload the rules. Large files are expected with hundreds of thousands of matching strings and 3-5 matching rules, each. For this reason, performance needs to be optimized - it needs to be able to handle large files and should be quick to detect changes in them too.

## Command-line options

Initial options requirements:
- directory from which to read all ".json" files for match configurations (default: /var/lib/stringsimile)
- configuration file to read (default /etc/stringsimile/stringsimile.yaml)
- path/filename for storing prometheus output (default /var/lib/node-exporter/stringsimile.prom with chmod 644 permissions)
- interval for writing prometheus output (seconds) (default: 15)
- take input from stdin (no default)
- take input from a filename (no default)
- kafka server/port for input match event data (no default server, but 9092 as default port)
- kafka topic for input match event data (no default)
- kafka identifier for input match data (default: stringsimile-$HOSTNAME)
- kafka pointer (default: start from pointer for the given kafka group ID. Optional: start from "now" aka "end", or numeric offset from end, like "10000" meaning 10000 events in the past?)
- json field to match in input kafka topic (default: .domain_name)
- kafka server/port for output topic (no default server, but 9092 as default port)
- filename for output instead of kafka or stdout (no default)
- use stdout for output instead of a file or kafka (no default)
- "report all" options meaning show result values for all input and all rules, even if they do not meet match thresholds. For testing only so that it is possible to see the numeric or other "result" data for individual rules as applied to input, even if they do not meet the minimum criteria for a match. (default: false)
- logfile location (default: /var/log/stringsimile)


All command line options except for config file name should have configuration file options to match (YAML?).

The "hangup" (HUP) signal should cause the system to close all sessions and re-read the directory containing the match configuration files. If possible, sessions should be kept open and operational rather than closing/restarting.

## Configuration / data transmission

Any/all fields in the input JSON object should be copied/transmitted to output JSON object, except that any results should be added as a new JSON object called "nameMatches" which is an array.

Magic fields at the top of each rule_set:
- split_target: break apart the target name based on "." zone cuts. Default: true. If "no", then evaluate the whole target string versus every name in each zone cut.
- ignore_tld: if split_target is "true" then ignore the rightmost part of the domain name (the top level domain) since that probably won't match and is a waste of compute resources.  Default: true.

## Prometheus metrics

The process should expose metrics, that can be exported to prometheus:
- version
- uptime
- memory used
- number of rules in memory
- number of strings to match in memory
- number of unique string_groupnames in memory
- input quantity of events from input methods (per input method)
- input bytes per input method
- output quantity of events
- output bytes
- quantity of rule matches with cardinality labels - string_groupname, string_match, rule_type
- maybe more if needed

## Methods

Preliminary methods for matching:

- Levenshtein Distance
> Best for: General string similarity, typo detection
> Example: "example.com" vs "exampl.com"

- IDN Confusables
> Best for: Detecting homograph attacks using Unicode
> Example: "example.com" vs "еxample.com" (with Cyrillic 'e')

- Soundex
> Best for: Phonetic matching of names
> Example: "example" vs "eggsample"

Damerau-Levenshtein
> Best for: Detecting keyboard typos with transposed characters
> Example: "example.com" vs "exmaple.com"

- Jaro
> Best for: Short string comparisons, name matching
> Example: "example.com" vs "exemple.com"

- Jaro-Winkler
> Best for: Strings that match from the beginning
> Example: "example.com" vs "examp1e.com"

- Hamming Distance
> Best for: Equal-length string comparison
> Example: "example.com" vs "exampl3.com"

- Metaphone
> Best for: Improved phonetic matching
> Example: "example" vs "eksampel"

- NYSIIS
> Best for: Name matching with better accuracy than Soundex
> Example: "example" vs "egzample"

- Match Rating
> Best for: Name comparison and coding
> Example: "example" vs "xample"

## Performance and libraries

Since performance is critical for the project, SIMD would be great to use where possible to speed things up. There are SIMD libraries for some of the models and they should be used wherever possible.
