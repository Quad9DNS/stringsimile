# stringsimile

Take a target string in JSON-structured stream (kafka) or file form and compare it against a large set of other strings using rules such as Levenshtein, Jaro, Soundex, IDN Confusables and more.

## Usage

Basic stdin/stdout usage:
```
stringsimile --input-from-stdin --output-to-stdout
```

For more information, use `--help`:
```
stringsimile --help
```

Check out the [example config file](./distribution/config.yaml) for options that can be configured in the configuration file.

## Features

Stringsimile supports different inputs, outputs and rules to use when comparing strings.

### Supported inputs

- stdin
- file
- kafka (when built with kafka support)

### Supported outputs

- stdout
- file
- kafka (when built with kafka support)

### Supported rules

- Levenshtein
- Jaro
- Jaro-Winkler
- IDN Confusables
- Damerau-Levenshtein
- Hamming
- Soundex
- Metaphone
- NYSIIS
- Match Rating

Check out the [example rules file](./distribution/rules/example.json) to see how they can be defined. You can also check out the included man pages (`man 5 stringsimile-rule-config`).

## Installing

Download your package from [releases](https://github.com/Quad9DNS/stringsimile/releases). We provide 3 different feature sets:

| Name        | Description                                                                     | Dependencies | Kafka supported |
|-------------|---------------------------------------------------------------------------------|--------------|-----------------|
| default     | Provides all features in a statically linked binary, no additional dependencies | -            | yes             |
| all-dynamic | Provides all features in a dynamically linked binary                            | librdkafka   | yes             |
| basic       | Provides the basic features in a statically linked binary                       | -            | no              |

We also provide `musl` and `gnu` builds. `gnu` builds require GLIBC 2.38 which is fairly recent, so `musl` can be used if you receive
```
version `GLIBC_2.38' not found (required by stringsimile)
```

### Debian

Install `librdkafka1` (if using dynamically linked build):
```
apt-get install librdkafka1
```

Download `.deb` release with your selected feature set and libc and install it:
```
unzip stringsimile-*-deb.zip
dpkg -i stringsimile_*.deb
```

### RPM

Install `librdkafka` (if using dynamically linked build):
```
dnf install librdkafka
```

Download `.deb` release with your selected feature set and libc and install it:
```
unzip stringsimile-*-rpm.zip
rpm -i stringsimile_*.rpm
```

### Binary

Install `librdkafka` if using dynamically linked build.

Download `binary` release with your selected feature set and run it. This build will be missing documentation and default config and ruleset.

### From source

The repository provides a `Makefile` that will install the binary and the documentation, as well as default configuration and rulesets:
```shell-session
$ git clone https://github.com/Quad9DNS/stringsimile/
$ cd stringsimile
$ make
# make install
```

## License
Stringsimile - tool for comparing target strings from JSON-structured streams against a large set of other strings using rules such as Levenshtein, Jaro, Soundex, IDN Confusables and more.
Copyright (C) 2025 Quad9 DNS

This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.

[AGPL-3.0](./LICENSE)
