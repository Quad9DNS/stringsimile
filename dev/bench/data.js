window.BENCHMARK_DATA = {
  "lastUpdate": 1771330685883,
  "repoUrl": "https://github.com/Quad9DNS/stringsimile",
  "entries": {
    "Benchmark": [
      {
        "commit": {
          "author": {
            "email": "dev@ensarsarajcic.com",
            "name": "Ensar Sarajčić",
            "username": "esensar"
          },
          "committer": {
            "email": "dev@ensarsarajcic.com",
            "name": "Ensar Sarajčić",
            "username": "esensar"
          },
          "distinct": true,
          "id": "1e4b55d74df666e78e9a63a539026567e9ec98d6",
          "message": "Disable benchmark CI results autopush on PRs",
          "timestamp": "2026-02-17T13:13:34+01:00",
          "tree_id": "d01f60bfd1bc8b0887fae218a82dee861034dfeb",
          "url": "https://github.com/Quad9DNS/stringsimile/commit/1e4b55d74df666e78e9a63a539026567e9ec98d6"
        },
        "date": 1771330685035,
        "tool": "cargo",
        "benches": [
          {
            "name": "confusables/confusables",
            "value": 85257,
            "range": "± 960",
            "unit": "ns/iter"
          },
          {
            "name": "levenshtein/levenshtein",
            "value": 114204,
            "range": "± 2310",
            "unit": "ns/iter"
          },
          {
            "name": "damerau_levenshtein/damerau_levenshtein",
            "value": 124460,
            "range": "± 1527",
            "unit": "ns/iter"
          },
          {
            "name": "hamming/hamming",
            "value": 1540,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "jaro/jaro",
            "value": 100389,
            "range": "± 1006",
            "unit": "ns/iter"
          },
          {
            "name": "jaro_winkler/jaro_winkler",
            "value": 99835,
            "range": "± 2002",
            "unit": "ns/iter"
          },
          {
            "name": "match_rating/match_rating",
            "value": 396817,
            "range": "± 1275",
            "unit": "ns/iter"
          },
          {
            "name": "metaphone_normal/metaphone_normal",
            "value": 48576,
            "range": "± 1371",
            "unit": "ns/iter"
          },
          {
            "name": "metaphone_double/metaphone_double",
            "value": 126457,
            "range": "± 315",
            "unit": "ns/iter"
          },
          {
            "name": "nysiis/nysiis",
            "value": 276605,
            "range": "± 741",
            "unit": "ns/iter"
          },
          {
            "name": "nysiis_strict/nysiis_strict",
            "value": 278941,
            "range": "± 678",
            "unit": "ns/iter"
          },
          {
            "name": "soundex/soundex",
            "value": 167878,
            "range": "± 383",
            "unit": "ns/iter"
          },
          {
            "name": "soundex_refined/soundex_refined",
            "value": 165104,
            "range": "± 1779",
            "unit": "ns/iter"
          },
          {
            "name": "all_rules_split_target_all/all_rules_split_target_all",
            "value": 9224838,
            "range": "± 54624",
            "unit": "ns/iter"
          },
          {
            "name": "all_rules_split_target_skip_tld/all_rules_split_target_skip_tld",
            "value": 9259441,
            "range": "± 69555",
            "unit": "ns/iter"
          },
          {
            "name": "all_rules_no_split_target/all_rules_no_split_target",
            "value": 9251139,
            "range": "± 32063",
            "unit": "ns/iter"
          },
          {
            "name": "stringsimile_service/processor",
            "value": 2983,
            "range": "± 11",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}