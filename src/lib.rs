#![deny(unsafe_code)]

//! Public convenience bindings and conformance models for Sigil plugins.
//!
//! The versioned WIT files remain authoritative. These Rust types are a
//! convenience surface and deliberately grant no authority on their own.

#[allow(unsafe_code, clippy::all, clippy::nursery, clippy::pedantic)]
pub mod host {
    wit_bindgen::generate!({
        path: "wit/sigil-host/1.0.0",
        world: "imports",
        generate_all,
    });
}

/// Language-neutral reference model for `sigil:sql/driver@0.1.0`.
pub mod sql {
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub struct ConnectOptions {
        pub endpoint: String,
        pub username_secret: String,
        pub password_secret: String,
        pub database: Option<String>,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum ErrorClass {
        Authentication,
        Server,
        Transport,
        Protocol,
        Encoding,
        Limit,
        Timeout,
        Closed,
        Unsupported,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Error {
        pub class: ErrorClass,
        pub vendor_code: Option<u32>,
        pub sqlstate: Option<String>,
        pub message: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case", tag = "kind", content = "value")]
    pub enum Cell {
        Null,
        Text(String),
        Bytes(Vec<u8>),
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Column {
        pub catalog: String,
        pub schema: String,
        pub table: String,
        pub original_table: String,
        pub name: String,
        pub original_name: String,
        pub vendor_type: u32,
        pub charset: u32,
        pub collation: u32,
        pub flags: u32,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Row {
        pub cells: Vec<Cell>,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct RowSet {
        pub columns: Vec<Column>,
        pub rows: Vec<Row>,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case", tag = "kind", content = "value")]
    pub enum QueryResult {
        Rows(RowSet),
        Command { affected_rows: u64 },
    }

    /// Minimal host-independent driver contract used by the conformance kit.
    pub trait Driver {
        type Connection: Connection;

        fn connect(&self, options: ConnectOptions) -> Result<Self::Connection, Error>;
    }

    pub trait Connection {
        fn query(&mut self, sql: &str) -> Result<QueryResult, Error>;
        fn close(&mut self);
    }

    /// Checked inclusive accumulation used by every published SQL limit.
    pub fn checked_accumulate(
        current: usize,
        increment: usize,
        maximum: usize,
    ) -> Result<usize, ErrorClass> {
        current
            .checked_add(increment)
            .filter(|value| *value <= maximum)
            .ok_or(ErrorClass::Limit)
    }

    /// Host-independent deterministic mock for binding and lifecycle tests.
    #[derive(Clone, Debug)]
    pub struct MockDriver {
        result: QueryResult,
    }

    impl MockDriver {
        #[must_use]
        pub const fn new(result: QueryResult) -> Self {
            Self { result }
        }
    }

    #[derive(Clone, Debug)]
    pub struct MockConnection {
        result: QueryResult,
        closed: bool,
    }

    impl Driver for MockDriver {
        type Connection = MockConnection;

        fn connect(&self, _options: ConnectOptions) -> Result<Self::Connection, Error> {
            Ok(MockConnection {
                result: self.result.clone(),
                closed: false,
            })
        }
    }

    impl Connection for MockConnection {
        fn query(&mut self, _sql: &str) -> Result<QueryResult, Error> {
            if self.closed {
                return Err(Error {
                    class: ErrorClass::Closed,
                    vendor_code: None,
                    sqlstate: None,
                    message: "connection is closed".to_owned(),
                });
            }
            Ok(self.result.clone())
        }

        fn close(&mut self) {
            self.closed = true;
        }
    }
}

/// A deterministic, authority-free host model for unit tests.
pub mod test_host {
    use std::collections::{BTreeMap, BTreeSet};

    #[derive(Clone, Debug, Default)]
    pub struct TestHost {
        pub secrets: BTreeMap<String, Vec<u8>>,
        pub endpoints: BTreeSet<String>,
        pub random: Vec<u8>,
        pub logs: Vec<(LogLevel, String)>,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum LogLevel {
        Debug,
        Info,
        Warn,
    }

    impl TestHost {
        pub fn secret(&self, name: &str) -> Option<&[u8]> {
            self.secrets.get(name).map(Vec::as_slice)
        }

        pub fn endpoint_granted(&self, name: &str) -> bool {
            self.endpoints.contains(name)
        }

        pub fn deterministic_bytes(&self, count: usize) -> Option<&[u8]> {
            self.random.get(..count)
        }

        pub fn log(&mut self, level: LogLevel, message: impl Into<String>) {
            self.logs.push((level, message.into()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::sql::{
        Cell, ConnectOptions, Connection as _, Driver as _, ErrorClass, MockDriver, QueryResult,
        checked_accumulate,
    };
    use super::test_host::{LogLevel, TestHost};

    #[test]
    fn test_host_is_closed_and_deterministic() {
        let mut host = TestHost {
            secrets: std::iter::once(("MYSQL_PASSWORD".to_owned(), b"secret".to_vec())).collect(),
            endpoints: std::iter::once("database".to_owned()).collect(),
            random: (0_u8..16).collect(),
            logs: Vec::new(),
        };
        assert_eq!(host.secret("MYSQL_PASSWORD"), Some(b"secret".as_slice()));
        assert_eq!(host.secret("OTHER"), None);
        assert!(host.endpoint_granted("database"));
        assert!(!host.endpoint_granted("internet"));
        assert_eq!(host.deterministic_bytes(4), Some([0, 1, 2, 3].as_slice()));
        assert_eq!(host.deterministic_bytes(17), None);
        host.log(LogLevel::Info, "ready");
        assert_eq!(host.logs, vec![(LogLevel::Info, "ready".to_owned())]);
    }

    #[test]
    fn language_neutral_sql_vectors_match_reference_types() {
        let value: serde_json::Value =
            serde_json::from_str(include_str!("../conformance/sql-0.1.0.json"))
                .expect("checked-in conformance JSON");
        assert_eq!(value["interface"], "sigil:sql/driver@0.1.0");
        assert_eq!(value["limits"]["sql_bytes"], 1_048_575);
        assert_eq!(value["limits"]["packet_bytes"], 1_048_576);
        assert_eq!(value["limits"]["aggregate_cell_bytes"], 8_388_608);
        assert_eq!(value["limits"]["label_raw_bytes"], 1_024);
        assert_eq!(value["limits"]["sanitized_error_bytes"], 8_192);
        assert_eq!(value["host_mapping"]["denied"], "outer-review");
        assert_eq!(value["host_mapping"]["limit"], "limit-no-partial-result");
        assert_eq!(value["lifecycle"]["retry"], false);
        assert_eq!(value["lifecycle"]["close"], "idempotent");
        let vectors = value["boundary_vectors"]
            .as_array()
            .expect("boundary vector array");
        assert_eq!(vectors.len(), 10);
        assert!(vectors.iter().all(|vector| {
            vector["maximum"] == "ok"
                && vector["maximum_plus_one"] == "limit"
                && vector["overflow"] == "limit"
        }));
        assert_eq!(
            serde_json::to_value(ErrorClass::Authentication).expect("serialize"),
            "authentication"
        );
        assert_eq!(
            serde_json::to_value(Cell::Bytes(vec![0, 255])).expect("serialize"),
            serde_json::json!({"kind": "bytes", "value": [0, 255]})
        );
    }

    #[test]
    fn every_reference_limit_is_checked_and_mock_lifecycle_is_closed() {
        for maximum in [1_024, 10_000, 100_000, 1_048_575, 1_048_576, 8_388_608] {
            assert_eq!(checked_accumulate(0, 0, maximum), Ok(0));
            assert_eq!(checked_accumulate(0, maximum, maximum), Ok(maximum));
            assert_eq!(
                checked_accumulate(maximum, 1, maximum),
                Err(ErrorClass::Limit)
            );
            assert_eq!(
                checked_accumulate(usize::MAX, 1, maximum),
                Err(ErrorClass::Limit)
            );
        }

        let driver = MockDriver::new(QueryResult::Command { affected_rows: 7 });
        let mut connection = driver
            .connect(ConnectOptions {
                endpoint: "database".to_owned(),
                username_secret: "MYSQL_USER".to_owned(),
                password_secret: "MYSQL_PASSWORD".to_owned(),
                database: Some("app".to_owned()),
            })
            .expect("mock connect");
        assert_eq!(
            connection.query("update t set x = 1"),
            Ok(QueryResult::Command { affected_rows: 7 })
        );
        connection.close();
        connection.close();
        assert_eq!(
            connection
                .query("select 1")
                .expect_err("closed query")
                .class,
            ErrorClass::Closed
        );
    }
}
