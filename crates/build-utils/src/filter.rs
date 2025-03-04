use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs};

use crate::path::PathWrapper;

type Folder = String;
type FilterMap = BTreeMap<Folder, Vec<String>>;

/// Filter to be applied on the tests files
#[derive(Deserialize, Default, Serialize)]
pub struct Filter {
    /// Mapping containing the directories and the files that should be skipped
    filename: FilterMap,
    /// Mapping containing the directories and the regex patterns that should be skipped
    regex: FilterMap,
    /// Mapping containing the directories and the specific tests that should be skipped
    #[serde(rename = "testname")]
    test_name: FilterMap,
}

impl Filter {
    pub fn load_file(path: &str) -> Result<Self, eyre::Error> {
        let filter = fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&filter)?)
    }

    /// Checks if the given path is inside the filter object
    pub fn is_skipped(&self, path: &PathWrapper, case_name: Option<String>) -> bool {
        let dir_name = path.parent().file_stem_to_string();
        let file_name = path.file_stem_to_string();

        let mut should_skip = self
            .filename
            .get(&dir_name)
            .map(|filtered_files| filtered_files.iter().any(|filename| filename == &file_name))
            .unwrap_or_default();

        should_skip |= self
            .regex
            .get(&dir_name)
            .map(|regexes| {
                regexes.iter().any(|regex| {
                    Regex::new(regex.as_str())
                        .expect("Error with regex pattern")
                        .is_match(&file_name)
                })
            })
            .unwrap_or_default();

        if let Some(case_name) = case_name {
            should_skip |= self
                .test_name
                .get(&dir_name)
                .map(|tests| tests.iter().any(|test| test == &case_name))
                .unwrap_or_default();
        }

        should_skip
    }

    /// Returns the difference in keys (folders) between the two filters
    pub fn diff(&self, rhs: &Self) -> Vec<Folder> {
        let mut diff = Vec::new();
        diff.append(&mut map_diff(&self.filename, &rhs.filename));
        diff.append(&mut map_diff(&self.regex, &rhs.regex));
        diff.append(&mut map_diff(&self.test_name, &rhs.test_name));
        diff
    }
}

fn map_diff(lhs: &FilterMap, rhs: &FilterMap) -> Vec<Folder> {
    let mut top = Vec::new();
    let diff = |top: &mut Vec<String>, lhs: &FilterMap, rhs: &FilterMap| {
        for (key, _) in lhs.iter() {
            if !rhs.contains_key(key) && !top.contains(key) {
                top.push(key.clone());
                continue;
            }
            let same = lhs
                .get(key)
                .unwrap()
                .iter()
                .zip(rhs.get(key).unwrap().iter())
                .all(|(lhs, rhs)| lhs == rhs);
            if !same && !top.contains(key) {
                top.push(key.clone());
            }
        }
    };

    diff(&mut top, lhs, rhs);
    diff(&mut top, rhs, lhs);

    top
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    #[ignore]
    fn test_filter_file() {
        let filter = Filter::load_file("../../blockchain-tests-skip.yml").unwrap();
        let path = PathWrapper::from(Path::new(
            "../../ef-testing/ethereum-tests/BlockchainTests/GeneralStateTests/stCallCreateCallCodeTest/Call1024PreCalls.json",
        ).to_path_buf());
        assert!(filter.is_skipped(&path, None));
    }

    #[test]
    fn test_filter_regex() {
        let filter = Filter::load_file("../../blockchain-tests-skip.yml").unwrap();
        let path = PathWrapper::from(Path::new(
            "../../ef-testing/ethereum-tests/BlockchainTests/GeneralStateTests/stBadOpcode/opc4DDiffPlaces.json",
        ).to_path_buf());
        assert!(filter.is_skipped(&path, None));
    }

    #[test]
    #[ignore]
    fn test_filter_test() {
        let filter = Filter::load_file("../../blockchain-tests-skip.yml").unwrap();
        let path = PathWrapper::from(Path::new(
            "../../ef-testing/ethereum-tests/BlockchainTests/GeneralStateTests/stTransactionTest/Opcodes_TransactionInit.json",
        ).to_path_buf());
        assert!(filter.is_skipped(
            &path,
            Some("Opcodes_TransactionInit_d111g0v0_Shanghai".to_string())
        ));
    }

    #[test]
    fn test_map_diff() {
        // Given
        let lhs: FilterMap = vec![
            ("a".to_string(), vec!["a".to_string()]),
            ("b".to_string(), vec!["b".to_string(), "b".to_string()]),
            (
                "c".to_string(),
                vec!["c".to_string(), "c".to_string(), "c".to_string()],
            ),
            (
                "e".to_string(),
                vec!["e".to_string(), "f".to_string(), "g".to_string()],
            ),
        ]
        .into_iter()
        .collect();
        let rhs: FilterMap = vec![
            ("a".to_string(), vec!["a".to_string()]),
            ("b".to_string(), vec!["b".to_string(), "d".to_string()]),
            (
                "c".to_string(),
                vec!["c".to_string(), "c".to_string(), "c".to_string()],
            ),
            (
                "d".to_string(),
                vec!["e".to_string(), "f".to_string(), "g".to_string()],
            ),
        ]
        .into_iter()
        .collect();

        // When
        let mut diff = map_diff(&lhs, &rhs);
        diff.sort();

        // Then
        let expected: Vec<Folder> = vec!["b".to_string(), "d".to_string(), "e".to_string()]
            .into_iter()
            .collect();

        assert_eq!(diff, expected)
    }
}
