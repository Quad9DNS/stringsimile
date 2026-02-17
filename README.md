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

Check out [the repository](https://github.com/Quad9DNS/stringsimile) for more information.

## Supported rules

Check out [rules benchmarks](benches.html) for a list of supported rules and their benchmark results.

## Benchmarks

Check out [full benchmark results](dev/bench) from latest commit on main branch.

## License
Stringsimile - tool for comparing target strings from JSON-structured streams against a large set of other strings using rules such as Levenshtein, Jaro, Soundex, IDN Confusables and more.
Copyright (C) 2025 Quad9 DNS

This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.

[AGPL-3.0](https://github.com/Quad9DNS/stringsimile/blob/main/LICENSE)
