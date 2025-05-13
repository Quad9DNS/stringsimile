use criterion::{Criterion, criterion_group, criterion_main};
use stringsimile_matcher::{
    rule::IntoGenericMatcherRule,
    rules::{
        confusables::ConfusablesRule,
        damerau_levenshtein::DamerauLevenshteinRule,
        hamming::HammingRule,
        jaro::JaroRule,
        jaro_winkler::JaroWinklerRule,
        levenshtein::LevenshteinRule,
        match_rating::MatchRatingRule,
        metaphone::{MetaphoneRule, MetaphoneRuleType},
        nysiis::NysiisRule,
        soundex::{SoundexRule, SoundexRuleType},
    },
    ruleset::{RuleSet, StringGroup},
};

const INPUT_DATA: [&str; 100] = [
    "bot-upload.s3.amazonaws.com.",
    "webapp.dtes.mh.gob.sv.",
    "binance.org.",
    "akmstatic.ml.youngjoygame.com.",
    "ar2fnu-inapps.appsflyersdk.com.",
    "activity.funshareapp.com.",
    "eafd-ffgov-phxr5b1-skype.elasticafd.msedge.azure.us.",
    "lhoefa.c3gm374dc.com.",
    "br.mia-assistance.com.",
    "123putlocker.pro.",
    "solvnow-qa-dwpcatalog.onbmc.com.",
    "informer.com.",
    "slt797.com.",
    "efcom-my.sharepoint.com.",
    "ks-livemate.pull.yximgs.com.",
    "ww7.pltraffic13.com.",
    "ambulatorial.siresp.saude.sp.gov.br.",
    "639167b0afdf4c92e0429c1747815fbf.azr.footprintdns.com.",
    "st-sysupgrade.vivoglobal.com.",
    "ox6mfe-conversions.appsflyersdk.com.",
    "ec2-13-229-211-58.ap-southeast-1.compute.amazonaws.com.",
    "ns2-38.azure-dns.net.",
    "stg-data-th.ads.heytapmobile.com.",
    "cndl.synology.cn.",
    "wlkc5a7bzhtm.l4.adsco.re.",
    "net.wac-0003.wac-msedge.net.",
    "thumbnailer.mixcloud.com.",
    "ubiabox-eu.s3-eu-central-1.amazonaws.com.",
    "nationalbankkenya-my.sharepoint.com.",
    "a4055.casalemedia.com.",
    "www.vlaamsnieuws.be.",
    "squirt.org.",
    "core.citrixworkspacesapi.net.",
    "spidersense.bendingspoons.com.",
    "www.octopus.com.",
    "01a52cfb-f390-466d-ac8e-5d1e74661789-netseer-ipaddr-assoc.xz.fbcdn.net.",
    "msit.loki.delve.office.com.",
    "e28e6857847a0a1826f4e968ab124f04.safeframe.googlesyndication.com.",
    "a437a4a4a31f42f8d04df053e05faa16.safeframe.googlesyndication.com.",
    "m.interml.yandex.kz.",
    "lhnir.carparts.com.",
    "solar4america.com.",
    "2a9271c4af83dc0879720dbdc36f3660.safeframe.googlesyndication.com.",
    "e5475204-66ed-4d76-a594-f7a890b5ade9-netseer-ipaddr-assoc.xz.fbcdn.net.",
    "cb8da3e6687142e6a3e9c42fb2b8ac09.fp.measure.office.com.",
    "09b151896a0029d35df7dbae8fedf5c8.safeframe.googlesyndication.com.",
    "ns1.cloud4wp-s8.com.",
    "c6f5a4d9-e61a-4a52-940b-46e00c4dc60e-netseer-ipaddr-assoc.xz.fbcdn.net.",
    "36f40b25-6f27-4a2e-8bff-8c22b0947e40-netseer-ipaddr-assoc.xz.fbcdn.net.",
    "e2b92a7d41cceba6d393c076570da65d.fp.measure.office.com.",
    "047d4dd2-481e-4939-a9ef-654c578cd9c4-netseer-ipaddr-assoc.xz.fbcdn.net.",
    "cslawchs.us6.my.auvik.com.",
    "yhydf0-dynamic-report-api.appsflyersdk.com.",
    "4e2f1229-146a-4488-a170-eabb61148d28-netseer-ipaddr-assoc.xz.fbcdn.net.",
    "endpoints.magicapple.tech.",
    "w30bxw-726-ppp.oss-ap-southeast-1.aliyuncs.com.",
    "rgslinuxfunctions01a240.blob.core.windows.net.",
    "covercraft.com.",
    "36252656f0a9cbd3f0cad2731bf2611a.safeframe.googlesyndication.com.",
    "3baedd0dadd24066912b1e077f72b0ad.fp.measure.office.com.",
    "barstardo.net.",
    "gospel-stream-service.churchofjesuschrist.org.",
    "6b92345d1e4d4390939a828afb0380df.fp.measure.office.com.",
    "7yyxv8-inapps.appsflyersdk.com.",
    "production-custom-ssl-41-elb-1347623642.us-east-1.elb.amazonaws.com.",
    "futbolenvivochile.com.",
    "5d56a70f-d419-4b70-9274-6992a5241129-netseer-ipaddr-assoc.xz.fbcdn.net.",
    "84a74bb02f1a425a9aaa60e794c92079.fp.measure.office.com.",
    "0d54125d9d3e4fdc91cf04cf67e4fd50.fp.measure.office.com.",
    "toa.tuchong.com.",
    "iso-bfrly-19.stanford.edu.",
    "priv-api.ellastvmax.com.",
    "cc6f74633509b24babd0ba14e7ba2094.safeframe.googlesyndication.com.",
    "eb080b7138cdb4b27b6ed2bf354a20b9.safeframe.googlesyndication.com.",
    "reservationgenie.com.",
    "cleanfoodcrush.ontraport.com.",
    "9efe81032a0d4e20b8a78f74fd530057.fp.measure.office.com.",
    "193811-ipv4v6w.farm.dprodmgd105.sharepointonline.com.akadns.net.",
    "yyy.duomian.com.",
    "22058-ipv4v6e.clump.dprodmgd105.aa-rt.sharepoint.com.",
    "48a55608e2b44997aa7085ed41f2d7f8.fp.measure.office.com.",
    "904d865390d2e90fae979d43a77e8727.safeframe.googlesyndication.com.",
    "32c3d3095ac4a7f421e97c0c75101da4.safeframe.googlesyndication.com.",
    "b12dc67d13cde5b403261679148b581f.safeframe.googlesyndication.com.",
    "5c507abe-0a76-4094-b010-675e06e2ef0e-netseer-ipaddr-assoc.xz.fbcdn.net.",
    "oceana.my.salesforce.com.",
    "f5daa159-afae-4728-865b-fce8b1752138-netseer-ipaddr-assoc.xz.fbcdn.net.",
    "786982b40cac8f4045631349c7aefb5e.safeframe.googlesyndication.com.",
    "akmcdn.ml.youngjoygame.com.",
    "183.186.95.23.in-addr.arpa.",
    "10bc6a7a-2e7f-43c4-9ec9-edce291dd84b-netseer-ipaddr-assoc.xz.fbcdn.net.",
    "a9b45543-e93e-4664-bb9d-1334543e8c07-netseer-ipaddr-assoc.xz.fbcdn.net.",
    "ok3-crtrs.oktaedge.okta.com.",
    "heroes-io.nextersglobal.com.",
    "deallinknet.com.",
    "a3vvqcp8z371p3-ats.iot.eu-central-1.amazonaws.com.",
    "3d4286dbd47a5c569a9d004e379edb4b.safeframe.googlesyndication.com.",
    "db4ee52548d0fc21f38f3d9f53ef3263.fp.measure.office.com.",
    "sgw-sg.c.huawei.com.",
    "basedintheory.com.",
];

macro_rules! bench_ruleset {
    (name = $rule_name:ident; builder { $builder:expr }) => {
        fn $rule_name(c: &mut Criterion) {
            let string_group = $builder;
            let mut group = c.benchmark_group(stringify!($rule_name));
            group.throughput(criterion::Throughput::Bytes(
                string_group
                    .rule_sets
                    .iter()
                    .map(|rule_set| {
                        INPUT_DATA
                            .iter()
                            .map(|input| input.len() as u64)
                            .sum::<u64>()
                            * rule_set.rules.len() as u64
                    })
                    .sum(),
            ));
            group.bench_function(stringify!($rule_name), |b| {
                b.iter(|| {
                    INPUT_DATA.iter().for_each(|input| {
                        let _ = string_group.generate_matches(input);
                    })
                });
            });
            group.finish();
        }
    };
}

bench_ruleset! {
    name = all_rules_split_target_all;
    builder {
        StringGroup::new("test_group".to_string(), vec![RuleSet {
            name:"test_ruleset".to_string(),
            string_match: "test.string.to.match".to_string(),
            split_target: true,
            ignore_tld: false,
            rules: vec![
                Box::new(ConfusablesRule.into_generic_matcher()),
                Box::new(LevenshteinRule { maximum_distance: 5 }.into_generic_matcher()),
                Box::new(DamerauLevenshteinRule { maximum_distance: 5 }.into_generic_matcher()),
                Box::new(HammingRule { maximum_distance: 5 }.into_generic_matcher()),
                Box::new(JaroRule { match_percent: 0.5 }.into_generic_matcher()),
                Box::new(JaroWinklerRule { match_percent: 0.5 }.into_generic_matcher()),
                Box::new(MatchRatingRule.into_generic_matcher()),
                Box::new(MetaphoneRule { metaphone_type: MetaphoneRuleType::Normal, max_code_length: Some(4) }.into_generic_matcher()),
                Box::new(MetaphoneRule { metaphone_type: MetaphoneRuleType::Double, max_code_length: Some(4) }.into_generic_matcher()),
                Box::new(NysiisRule::new(false).into_generic_matcher()),
                Box::new(NysiisRule::new(true).into_generic_matcher()),
                Box::new(SoundexRule { soundex_type: SoundexRuleType::Normal, minimum_similarity: 5 }.into_generic_matcher()),
                Box::new(SoundexRule { soundex_type: SoundexRuleType::Refined, minimum_similarity: 5 }.into_generic_matcher()),
            ]
        }])
    }
}

bench_ruleset! {
    name = all_rules_split_target_skip_tld;
    builder {
        StringGroup::new("test_group".to_string(), vec![RuleSet {
            name:"test_ruleset".to_string(),
            string_match: "test.string.to.match".to_string(),
            split_target: true,
            ignore_tld: true,
            rules: vec![
                Box::new(ConfusablesRule.into_generic_matcher()),
                Box::new(LevenshteinRule { maximum_distance: 5 }.into_generic_matcher()),
                Box::new(DamerauLevenshteinRule { maximum_distance: 5 }.into_generic_matcher()),
                Box::new(HammingRule { maximum_distance: 5 }.into_generic_matcher()),
                Box::new(JaroRule { match_percent: 0.5 }.into_generic_matcher()),
                Box::new(JaroWinklerRule { match_percent: 0.5 }.into_generic_matcher()),
                Box::new(MatchRatingRule.into_generic_matcher()),
                Box::new(MetaphoneRule { metaphone_type: MetaphoneRuleType::Normal, max_code_length: Some(4) }.into_generic_matcher()),
                Box::new(MetaphoneRule { metaphone_type: MetaphoneRuleType::Double, max_code_length: Some(4) }.into_generic_matcher()),
                Box::new(NysiisRule::new(false).into_generic_matcher()),
                Box::new(NysiisRule::new(true).into_generic_matcher()),
                Box::new(SoundexRule { soundex_type: SoundexRuleType::Normal, minimum_similarity: 5 }.into_generic_matcher()),
                Box::new(SoundexRule { soundex_type: SoundexRuleType::Refined, minimum_similarity: 5 }.into_generic_matcher()),
            ]
        }])
    }
}

bench_ruleset! {
    name = all_rules_no_split_target;
    builder {
        StringGroup::new("test_group".to_string(), vec![RuleSet {
            name:"test_ruleset".to_string(),
            string_match: "test.string.to.match".to_string(),
            split_target: true,
            ignore_tld: true,
            rules: vec![
                Box::new(ConfusablesRule.into_generic_matcher()),
                Box::new(LevenshteinRule { maximum_distance: 5 }.into_generic_matcher()),
                Box::new(DamerauLevenshteinRule { maximum_distance: 5 }.into_generic_matcher()),
                Box::new(HammingRule { maximum_distance: 5 }.into_generic_matcher()),
                Box::new(JaroRule { match_percent: 0.5 }.into_generic_matcher()),
                Box::new(JaroWinklerRule { match_percent: 0.5 }.into_generic_matcher()),
                Box::new(MatchRatingRule.into_generic_matcher()),
                Box::new(MetaphoneRule { metaphone_type: MetaphoneRuleType::Normal, max_code_length: Some(4) }.into_generic_matcher()),
                Box::new(MetaphoneRule { metaphone_type: MetaphoneRuleType::Double, max_code_length: Some(4) }.into_generic_matcher()),
                Box::new(NysiisRule::new(false).into_generic_matcher()),
                Box::new(NysiisRule::new(true).into_generic_matcher()),
                Box::new(SoundexRule { soundex_type: SoundexRuleType::Normal, minimum_similarity: 5 }.into_generic_matcher()),
                Box::new(SoundexRule { soundex_type: SoundexRuleType::Refined, minimum_similarity: 5 }.into_generic_matcher()),
            ]
        }])
    }
}

criterion_group!(
    benches,
    all_rules_split_target_all,
    all_rules_split_target_skip_tld,
    all_rules_no_split_target
);
criterion_main!(benches);
