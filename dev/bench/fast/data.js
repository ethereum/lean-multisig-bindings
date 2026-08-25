window.BENCHMARK_DATA = {
  "lastUpdate": 1787678000068,
  "repoUrl": "https://github.com/ethereum/lean-multisig-bindings",
  "entries": {
    "Lighthouse comparison (fast)": [
      {
        "commit": {
          "author": {
            "email": "kevtheappdev@gmail.com",
            "name": "kevaundray",
            "username": "kevaundray"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6a3da3a7121cfa9198849353b8f98c2265254009",
          "message": "feat: benchmark leanMultisig against Lighthouse BLS (#17)",
          "timestamp": "2026-08-25T17:58:31+01:00",
          "tree_id": "884937cdedcdf9feeaf2a5a08a848f38b5f2309a",
          "url": "https://github.com/ethereum/lean-multisig-bindings/commit/6a3da3a7121cfa9198849353b8f98c2265254009"
        },
        "date": 1787677996250,
        "tool": "cargo",
        "benches": [
          {
            "name": "key_creation/lean",
            "value": 5937262,
            "range": "± 658840",
            "unit": "ns/iter"
          },
          {
            "name": "key_creation/lighthouse",
            "value": 843,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "public_key/lean",
            "value": 28,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "public_key/lighthouse",
            "value": 113260,
            "range": "± 659",
            "unit": "ns/iter"
          },
          {
            "name": "sign/lean",
            "value": 2830800,
            "range": "± 38870",
            "unit": "ns/iter"
          },
          {
            "name": "sign/lighthouse",
            "value": 409598,
            "range": "± 1382",
            "unit": "ns/iter"
          },
          {
            "name": "raw_signature_serialize/lean",
            "value": 490,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "raw_signature_serialize/lighthouse",
            "value": 154,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "raw_signature_deserialize/lean",
            "value": 802,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "raw_signature_deserialize/lighthouse",
            "value": 33908,
            "range": "± 79",
            "unit": "ns/iter"
          },
          {
            "name": "single_verify/lean",
            "value": 461740,
            "range": "± 2777",
            "unit": "ns/iter"
          },
          {
            "name": "single_verify/lighthouse",
            "value": 1047021,
            "range": "± 48307",
            "unit": "ns/iter"
          },
          {
            "name": "lighthouse_same_claim_aggregate/1",
            "value": 1575,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "lighthouse_same_claim_aggregate/8",
            "value": 11991,
            "range": "± 62",
            "unit": "ns/iter"
          },
          {
            "name": "lighthouse_same_claim_aggregate/16",
            "value": 23867,
            "range": "± 66",
            "unit": "ns/iter"
          },
          {
            "name": "lighthouse_same_claim_aggregate/32",
            "value": 47756,
            "range": "± 675",
            "unit": "ns/iter"
          },
          {
            "name": "lighthouse_same_claim_aggregate/64",
            "value": 95182,
            "range": "± 215",
            "unit": "ns/iter"
          },
          {
            "name": "lighthouse_same_claim_aggregate/128",
            "value": 190209,
            "range": "± 918",
            "unit": "ns/iter"
          },
          {
            "name": "lighthouse_same_claim_aggregate/256",
            "value": 380283,
            "range": "± 5756",
            "unit": "ns/iter"
          },
          {
            "name": "lighthouse_same_claim_aggregate/512",
            "value": 760779,
            "range": "± 5231",
            "unit": "ns/iter"
          },
          {
            "name": "lighthouse_same_claim_verify/1",
            "value": 955744,
            "range": "± 49492",
            "unit": "ns/iter"
          },
          {
            "name": "lighthouse_same_claim_verify/8",
            "value": 1102218,
            "range": "± 46712",
            "unit": "ns/iter"
          },
          {
            "name": "lighthouse_same_claim_verify/16",
            "value": 967545,
            "range": "± 31664",
            "unit": "ns/iter"
          },
          {
            "name": "lighthouse_same_claim_verify/32",
            "value": 1021750,
            "range": "± 58707",
            "unit": "ns/iter"
          },
          {
            "name": "lighthouse_same_claim_verify/64",
            "value": 1009699,
            "range": "± 122639",
            "unit": "ns/iter"
          },
          {
            "name": "lighthouse_same_claim_verify/128",
            "value": 1134805,
            "range": "± 62072",
            "unit": "ns/iter"
          },
          {
            "name": "lighthouse_same_claim_verify/256",
            "value": 1145279,
            "range": "± 58328",
            "unit": "ns/iter"
          },
          {
            "name": "lighthouse_same_claim_verify/512",
            "value": 1370990,
            "range": "± 67544",
            "unit": "ns/iter"
          },
          {
            "name": "lighthouse_distinct_claim_aggregate/1",
            "value": 1574,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "lighthouse_distinct_claim_aggregate/8",
            "value": 11984,
            "range": "± 336",
            "unit": "ns/iter"
          },
          {
            "name": "lighthouse_distinct_claim_aggregate/16",
            "value": 23863,
            "range": "± 131",
            "unit": "ns/iter"
          },
          {
            "name": "lighthouse_distinct_claim_verify/1",
            "value": 966256,
            "range": "± 29325",
            "unit": "ns/iter"
          },
          {
            "name": "lighthouse_distinct_claim_verify/8",
            "value": 2505464,
            "range": "± 13261",
            "unit": "ns/iter"
          },
          {
            "name": "lighthouse_distinct_claim_verify/16",
            "value": 3993725,
            "range": "± 38843",
            "unit": "ns/iter"
          },
          {
            "name": "lighthouse_signature_sets_verify/1",
            "value": 1461948,
            "range": "± 28007",
            "unit": "ns/iter"
          },
          {
            "name": "lighthouse_signature_sets_verify/8",
            "value": 3429287,
            "range": "± 24813",
            "unit": "ns/iter"
          },
          {
            "name": "lighthouse_signature_sets_verify/16",
            "value": 5950444,
            "range": "± 61999",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}