//! CIDR rule implementation

use ipnet::IpNet;
use std::{fmt::Debug, io::Error, net::IpAddr};

use serde::{Deserialize, Serialize};

use crate::{
    MatcherResult,
    rule::{MatcherResultRuleMetadataExt, MatcherRule, RuleMetadata},
};

/// Rule
#[derive(Debug, Clone)]
pub struct CidrRule {
    net: IpNet,
}

impl CidrRule {
    /// Creates a new instance of [`CidrRule`], with the provided address/network.
    pub fn new(net: IpNet) -> Self {
        Self { net }
    }
}

/// metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CidrMetadata;

impl MatcherRule for CidrRule {
    type OutputMetadata = CidrMetadata;
    type Error = Error;

    fn match_rule(
        &self,
        input_str: &str,
        _target_str: &str,
    ) -> MatcherResult<Self::OutputMetadata, Self::Error> {
        let Ok(addr) = input_str.parse::<IpAddr>() else {
            return MatcherResult::new_no_match(CidrMetadata);
        };
        if self.net.contains(&addr) {
            MatcherResult::new_match(CidrMetadata)
        } else {
            MatcherResult::new_no_match(CidrMetadata)
        }
    }
}

impl RuleMetadata for CidrMetadata {
    const RULE_NAME: &str = "cidr";
}

#[cfg(test)]
mod tests {
    use crate::rule::MatcherResultExt;

    use super::*;

    #[test]
    fn simple_example() {
        let rule = CidrRule::new("192.168.0.0/24".parse().unwrap());

        let result = rule.match_rule("192.168.0.1", "whatever");
        assert!(result.is_match());
        let result = rule.match_rule("192.168.0.30", "whatever");
        assert!(result.is_match());
        let result = rule.match_rule("192.168.1.30", "whatever");
        assert!(!result.is_match());
    }
}
