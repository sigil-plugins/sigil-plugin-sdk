//! Provider-neutral reference model and deterministic conformance harness for
//! `sigil:sql/driver@0.2.0`.
//!
//! The canonical WIT remains authoritative. These types make its lossless
//! value, result-bound, and single-session rules executable without choosing a
//! database protocol or granting host authority.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use serde::{Deserialize, Serialize};

/// Exact nominal interface selected by a SQL 0.2 plugin manifest.
pub const ENTRYPOINT: &str = "sigil:sql/driver@0.2.0";

/// Caller-selected additional ceilings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ConnectOptions {
    pub endpoint: String,
    pub username_secret: String,
    pub password_secret: String,
    pub database: Option<String>,
    pub max_rows: Option<u32>,
    pub max_result_bytes: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorClass {
    Invalid,
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
#[serde(rename_all = "kebab-case")]
pub struct Error {
    pub class: ErrorClass,
    pub vendor_code: Option<u32>,
    pub sqlstate: Option<String>,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ColumnType {
    Null,
    Signed,
    Unsigned,
    Floating,
    Decimal,
    Text,
    Bytes,
    Temporal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TemporalType {
    Date,
    Time,
    TimeWithTimeZone,
    Datetime,
    Timestamp,
    TimestampWithTimeZone,
    Interval,
    Year,
    Vendor,
}

/// Exact tagged SQL value. The string arms preserve their server lexeme.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "tag", content = "value")]
pub enum Cell {
    Null,
    Signed(i64),
    Unsigned(u64),
    Floating(f64),
    Decimal(String),
    Text(String),
    Bytes(Vec<u8>),
    Temporal(String),
}

impl Cell {
    const fn column_type(&self) -> Option<ColumnType> {
        match self {
            Self::Null => None,
            Self::Signed(_) => Some(ColumnType::Signed),
            Self::Unsigned(_) => Some(ColumnType::Unsigned),
            Self::Floating(_) => Some(ColumnType::Floating),
            Self::Decimal(_) => Some(ColumnType::Decimal),
            Self::Text(_) => Some(ColumnType::Text),
            Self::Bytes(_) => Some(ColumnType::Bytes),
            Self::Temporal(_) => Some(ColumnType::Temporal),
        }
    }

    fn logical_bytes(&self) -> Result<u64, ErrorClass> {
        match self {
            Self::Null => Ok(0),
            Self::Signed(_) | Self::Unsigned(_) | Self::Floating(_) => Ok(8),
            Self::Decimal(value) | Self::Text(value) | Self::Temporal(value) => {
                u64::try_from(value.len()).map_err(|_error| ErrorClass::Limit)
            }
            Self::Bytes(value) => u64::try_from(value.len()).map_err(|_error| ErrorClass::Limit),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
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
    #[serde(rename = "type")]
    pub column_type: ColumnType,
    pub temporal_type: Option<TemporalType>,
}

impl Column {
    fn logical_bytes(&self) -> Result<u64, ErrorClass> {
        [
            &self.catalog,
            &self.schema,
            &self.table,
            &self.original_table,
            &self.name,
            &self.original_name,
        ]
        .into_iter()
        .try_fold(0_u64, |total, value| {
            checked_add(
                total,
                u64::try_from(value.len()).map_err(|_error| ErrorClass::Limit)?,
            )
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Row {
    pub cells: Vec<Cell>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RowSet {
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CommandResult {
    pub affected_rows: u64,
    pub last_insert_id: Option<u64>,
    pub warnings: u32,
}

/// One layer of result ceilings. `None` means this layer imposes no ceiling.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResultLimits {
    pub max_rows: Option<u32>,
    pub max_result_bytes: Option<u64>,
}

impl ResultLimits {
    /// Intersect two authority layers. A larger value can never widen a
    /// smaller existing ceiling.
    #[must_use]
    pub fn intersect(self, other: Self) -> Self {
        Self {
            max_rows: minimum(self.max_rows, other.max_rows),
            max_result_bytes: minimum(self.max_result_bytes, other.max_result_bytes),
        }
    }

    #[must_use]
    pub const fn from_options(options: &ConnectOptions) -> Self {
        Self {
            max_rows: options.max_rows,
            max_result_bytes: options.max_result_bytes,
        }
    }
}

fn minimum<T: Ord + Copy>(left: Option<T>, right: Option<T>) -> Option<T> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn checked_add(left: u64, right: u64) -> Result<u64, ErrorClass> {
    left.checked_add(right).ok_or(ErrorClass::Limit)
}

fn error(class: ErrorClass, message: &str) -> Error {
    Error {
        class,
        vendor_code: None,
        sqlstate: None,
        message: message.to_owned(),
    }
}

fn validate_row_set(row_set: &RowSet) -> Result<(), ErrorClass> {
    for column in &row_set.columns {
        let temporal = column.column_type == ColumnType::Temporal;
        if temporal != column.temporal_type.is_some() {
            return Err(ErrorClass::Protocol);
        }
    }
    for row in &row_set.rows {
        if row.cells.len() != row_set.columns.len() {
            return Err(ErrorClass::Protocol);
        }
        for (cell, column) in row.cells.iter().zip(&row_set.columns) {
            if cell
                .column_type()
                .is_some_and(|cell_type| cell_type != column.column_type)
            {
                return Err(ErrorClass::Protocol);
            }
        }
    }
    Ok(())
}

/// Portable logical-byte count defined by the SQL 0.2 contract.
pub fn logical_row_set_bytes(row_set: &RowSet) -> Result<u64, ErrorClass> {
    validate_row_set(row_set)?;
    let metadata = row_set.columns.iter().try_fold(0_u64, |total, column| {
        checked_add(total, column.logical_bytes()?)
    })?;
    row_set.rows.iter().try_fold(metadata, |total, row| {
        row.cells.iter().try_fold(total, |row_total, cell| {
            checked_add(row_total, cell.logical_bytes()?)
        })
    })
}

/// Portable logical-byte count for one command result.
#[must_use]
pub const fn logical_command_bytes(result: &CommandResult) -> u64 {
    if result.last_insert_id.is_some() {
        20
    } else {
        12
    }
}

/// Consume and admit one complete row set or return one error with no partial
/// result available to the caller.
pub fn admit_row_set(row_set: RowSet, limits: ResultLimits) -> Result<RowSet, Error> {
    let rows = u32::try_from(row_set.rows.len())
        .map_err(|_error| error(ErrorClass::Limit, "row count exceeds the result ceiling"))?;
    if limits.max_rows.is_some_and(|maximum| rows > maximum) {
        return Err(error(
            ErrorClass::Limit,
            "row count exceeds the result ceiling",
        ));
    }
    let bytes = logical_row_set_bytes(&row_set).map_err(|class| {
        error(
            class,
            if class == ErrorClass::Protocol {
                "row metadata and cells contradict the SQL contract"
            } else {
                "result byte accounting overflowed"
            },
        )
    })?;
    if limits
        .max_result_bytes
        .is_some_and(|maximum| bytes > maximum)
    {
        return Err(error(
            ErrorClass::Limit,
            "result bytes exceed the result ceiling",
        ));
    }
    Ok(row_set)
}

/// Consume and admit one complete command or return one error with no command
/// metadata available to the caller.
pub fn admit_command(result: CommandResult, limits: ResultLimits) -> Result<CommandResult, Error> {
    if limits
        .max_result_bytes
        .is_some_and(|maximum| logical_command_bytes(&result) > maximum)
    {
        return Err(error(
            ErrorClass::Limit,
            "command bytes exceed the result ceiling",
        ));
    }
    Ok(result)
}

pub trait Driver {
    type Connection: Connection;

    fn connect(&self, options: ConnectOptions) -> Result<Self::Connection, Error>;
}

pub trait Connection {
    fn query(&mut self, sql: &str) -> Result<RowSet, Error>;
    fn exec(&mut self, sql: &str) -> Result<CommandResult, Error>;
    fn close(&mut self);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptCall {
    Query(String),
    Exec(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ScriptOutcome {
    Rows(RowSet),
    Command(CommandResult),
    Error(Error),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScriptStep {
    pub call: ScriptCall,
    pub outcome: ScriptOutcome,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub connections: usize,
    pub calls: usize,
    pub close_events: usize,
}

/// Shared observation handle proving that one scripted connection served a
/// whole transcript and was closed exactly once, including implicit drop.
#[derive(Clone, Debug, Default)]
pub struct SessionProbe(Rc<RefCell<SessionSnapshot>>);

impl SessionProbe {
    #[must_use]
    pub fn snapshot(&self) -> SessionSnapshot {
        *self.0.borrow()
    }
}

/// A deterministic provider-neutral driver. It consumes an exact ordered
/// transcript and never retries, reconnects, or replays a step.
#[derive(Clone, Debug)]
pub struct ScriptedDriver {
    steps: Vec<ScriptStep>,
    driver_limits: ResultLimits,
    operator_limits: ResultLimits,
    probe: SessionProbe,
}

impl ScriptedDriver {
    #[must_use]
    pub fn new(
        steps: Vec<ScriptStep>,
        driver_limits: ResultLimits,
        operator_limits: ResultLimits,
    ) -> Self {
        Self {
            steps,
            driver_limits,
            operator_limits,
            probe: SessionProbe::default(),
        }
    }

    #[must_use]
    pub fn probe(&self) -> SessionProbe {
        self.probe.clone()
    }
}

impl Driver for ScriptedDriver {
    type Connection = ScriptedConnection;

    fn connect(&self, options: ConnectOptions) -> Result<Self::Connection, Error> {
        if options.endpoint.is_empty()
            || options.username_secret.is_empty()
            || options.password_secret.is_empty()
        {
            return Err(error(
                ErrorClass::Invalid,
                "endpoint and secret names must not be empty",
            ));
        }
        self.probe.0.borrow_mut().connections += 1;
        Ok(ScriptedConnection {
            steps: self.steps.clone().into(),
            limits: self
                .driver_limits
                .intersect(self.operator_limits)
                .intersect(ResultLimits::from_options(&options)),
            probe: self.probe.clone(),
            closed: false,
        })
    }
}

#[derive(Debug)]
pub struct ScriptedConnection {
    steps: VecDeque<ScriptStep>,
    limits: ResultLimits,
    probe: SessionProbe,
    closed: bool,
}

impl ScriptedConnection {
    #[must_use]
    pub fn remaining_steps(&self) -> usize {
        self.steps.len()
    }

    fn close_once(&mut self) {
        if !self.closed {
            self.closed = true;
            self.probe.0.borrow_mut().close_events += 1;
        }
    }

    fn take_step(&mut self, call: &ScriptCall) -> Result<ScriptOutcome, Error> {
        if self.closed {
            return Err(error(ErrorClass::Closed, "connection is closed"));
        }
        self.probe.0.borrow_mut().calls += 1;
        let Some(step) = self.steps.pop_front() else {
            self.close_once();
            return Err(error(
                ErrorClass::Protocol,
                "the conformance transcript has no matching call",
            ));
        };
        if &step.call != call {
            self.close_once();
            return Err(error(
                ErrorClass::Protocol,
                "the conformance transcript call order differs",
            ));
        }
        Ok(step.outcome)
    }

    fn finish_error<T>(&mut self, failure: Error) -> Result<T, Error> {
        if matches!(
            failure.class,
            ErrorClass::Transport
                | ErrorClass::Protocol
                | ErrorClass::Encoding
                | ErrorClass::Limit
                | ErrorClass::Timeout
        ) {
            self.close_once();
        }
        Err(failure)
    }
}

impl Connection for ScriptedConnection {
    fn query(&mut self, sql: &str) -> Result<RowSet, Error> {
        match self.take_step(&ScriptCall::Query(sql.to_owned()))? {
            ScriptOutcome::Rows(rows) => match admit_row_set(rows, self.limits) {
                Ok(rows) => Ok(rows),
                Err(failure) => self.finish_error(failure),
            },
            ScriptOutcome::Command(_command) => self.finish_error(error(
                ErrorClass::Unsupported,
                "query received a command response",
            )),
            ScriptOutcome::Error(failure) => self.finish_error(failure),
        }
    }

    fn exec(&mut self, sql: &str) -> Result<CommandResult, Error> {
        match self.take_step(&ScriptCall::Exec(sql.to_owned()))? {
            ScriptOutcome::Command(command) => match admit_command(command, self.limits) {
                Ok(command) => Ok(command),
                Err(failure) => self.finish_error(failure),
            },
            ScriptOutcome::Rows(_rows) => self.finish_error(error(
                ErrorClass::Unsupported,
                "exec received a row response",
            )),
            ScriptOutcome::Error(failure) => self.finish_error(failure),
        }
    }

    fn close(&mut self) {
        self.close_once();
    }
}

impl Drop for ScriptedConnection {
    fn drop(&mut self) {
        self.close_once();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(max_rows: Option<u32>, max_result_bytes: Option<u64>) -> ConnectOptions {
        ConnectOptions {
            endpoint: "database".to_owned(),
            username_secret: "SQL_USER".to_owned(),
            password_secret: "SQL_PASSWORD".to_owned(),
            database: Some("app".to_owned()),
            max_rows,
            max_result_bytes,
        }
    }

    fn column(name: &str, column_type: ColumnType, temporal_type: Option<TemporalType>) -> Column {
        Column {
            catalog: String::new(),
            schema: String::new(),
            table: "t".to_owned(),
            original_table: "t".to_owned(),
            name: name.to_owned(),
            original_name: name.to_owned(),
            vendor_type: 0,
            charset: 0,
            collation: 0,
            flags: 0,
            column_type,
            temporal_type,
        }
    }

    fn one_signed(value: i64) -> RowSet {
        RowSet {
            columns: vec![column("value", ColumnType::Signed, None)],
            rows: vec![Row {
                cells: vec![Cell::Signed(value)],
            }],
        }
    }

    fn command(affected_rows: u64, last_insert_id: Option<u64>, warnings: u32) -> CommandResult {
        CommandResult {
            affected_rows,
            last_insert_id,
            warnings,
        }
    }

    #[test]
    fn every_typed_cell_serializes_without_erasing_identity() {
        let values = [
            Cell::Null,
            Cell::Signed(i64::MIN),
            Cell::Unsigned(u64::MAX),
            Cell::Floating(-0.0),
            Cell::Decimal("001.2300".to_owned()),
            Cell::Text("snowman ☃".to_owned()),
            Cell::Bytes(vec![0, 255, 128]),
            Cell::Temporal("2026-08-30 12:34:56.000001+05:45".to_owned()),
        ];
        let encoded = values
            .iter()
            .map(|value| serde_json::to_value(value).expect("serialize typed cell"))
            .collect::<Vec<_>>();
        assert_eq!(encoded[0], serde_json::json!({"tag": "null"}));
        assert_eq!(encoded[1]["value"], i64::MIN);
        assert_eq!(encoded[2]["value"].as_u64(), Some(u64::MAX));
        assert_eq!(encoded[3]["tag"], "floating");
        let Cell::Floating(negative_zero) = values[3] else {
            panic!("floating fixture changed tag");
        };
        assert_eq!(negative_zero.to_bits(), (-0.0_f64).to_bits());
        assert_eq!(encoded[4]["value"], "001.2300");
        assert_eq!(encoded[5]["value"], "snowman ☃");
        assert_eq!(encoded[6]["value"], serde_json::json!([0, 255, 128]));
        assert_eq!(encoded[7]["value"], "2026-08-30 12:34:56.000001+05:45");
    }

    #[test]
    fn logical_accounting_is_portable_and_rejects_type_contradictions() {
        let temporal = "2026-08-30 12:34:56.000001+05:45";
        let rows = RowSet {
            columns: vec![
                column("id", ColumnType::Signed, None),
                column("note", ColumnType::Text, None),
                column(
                    "when",
                    ColumnType::Temporal,
                    Some(TemporalType::TimestampWithTimeZone),
                ),
            ],
            rows: vec![Row {
                cells: vec![
                    Cell::Signed(7),
                    Cell::Null,
                    Cell::Temporal(temporal.to_owned()),
                ],
            }],
        };
        let metadata_bytes = rows
            .columns
            .iter()
            .map(|item| item.logical_bytes().expect("metadata bytes"))
            .sum::<u64>();
        assert_eq!(
            logical_row_set_bytes(&rows),
            Ok(metadata_bytes + 8 + u64::try_from(temporal.len()).expect("temporal length"))
        );
        assert_eq!(logical_command_bytes(&command(0, None, 0)), 12);
        assert_eq!(logical_command_bytes(&command(u64::MAX, Some(0), 2)), 20);

        let mut wrong_width = rows.clone();
        wrong_width.rows[0].cells.pop();
        assert_eq!(
            logical_row_set_bytes(&wrong_width),
            Err(ErrorClass::Protocol)
        );

        let mut wrong_type = rows.clone();
        wrong_type.rows[0].cells[0] = Cell::Text("7".to_owned());
        assert_eq!(
            logical_row_set_bytes(&wrong_type),
            Err(ErrorClass::Protocol)
        );

        let mut missing_temporal_type = rows.clone();
        missing_temporal_type.columns[2].temporal_type = None;
        assert_eq!(
            logical_row_set_bytes(&missing_temporal_type),
            Err(ErrorClass::Protocol)
        );

        let mut unexpected_temporal_type = rows;
        unexpected_temporal_type.columns[0].temporal_type = Some(TemporalType::Vendor);
        assert_eq!(
            logical_row_set_bytes(&unexpected_temporal_type),
            Err(ErrorClass::Protocol)
        );
        assert_eq!(checked_add(u64::MAX, 1), Err(ErrorClass::Limit));
    }

    #[test]
    fn caller_limits_only_lower_driver_and_operator_ceilings() {
        let driver = ResultLimits {
            max_rows: Some(10_000),
            max_result_bytes: Some(8_388_608),
        };
        let operator = ResultLimits {
            max_rows: Some(5_000),
            max_result_bytes: Some(4_194_304),
        };
        let widened = ResultLimits::from_options(&options(Some(u32::MAX), Some(u64::MAX)));
        assert_eq!(driver.intersect(operator).intersect(widened), operator);

        let zero = ResultLimits::from_options(&options(Some(0), Some(0)));
        assert_eq!(
            driver.intersect(operator).intersect(zero),
            ResultLimits {
                max_rows: Some(0),
                max_result_bytes: Some(0),
            }
        );

        let rows = one_signed(7);
        let exact_bytes = logical_row_set_bytes(&rows).expect("logical bytes");
        assert!(
            admit_row_set(
                rows.clone(),
                ResultLimits {
                    max_rows: Some(1),
                    max_result_bytes: Some(exact_bytes),
                },
            )
            .is_ok()
        );
        let byte_error = admit_row_set(
            rows.clone(),
            ResultLimits {
                max_rows: Some(1),
                max_result_bytes: Some(exact_bytes - 1),
            },
        )
        .expect_err("maximum plus one logical byte must fail without output");
        assert_eq!(byte_error.class, ErrorClass::Limit);
        let row_error = admit_row_set(
            rows,
            ResultLimits {
                max_rows: Some(0),
                max_result_bytes: None,
            },
        )
        .expect_err("one row beyond a zero ceiling must fail without output");
        assert_eq!(row_error.class, ErrorClass::Limit);

        assert!(
            admit_command(
                command(0, None, 0),
                ResultLimits {
                    max_rows: None,
                    max_result_bytes: Some(12),
                },
            )
            .is_ok()
        );
        assert_eq!(
            admit_command(
                command(0, Some(0), 0),
                ResultLimits {
                    max_rows: None,
                    max_result_bytes: Some(19),
                },
            )
            .expect_err("twenty-byte command must not return a short success")
            .class,
            ErrorClass::Limit
        );

        let empty = RowSet {
            columns: Vec::new(),
            rows: Vec::new(),
        };
        assert_eq!(
            admit_row_set(
                empty.clone(),
                ResultLimits {
                    max_rows: Some(0),
                    max_result_bytes: Some(0),
                },
            ),
            Ok(empty)
        );
    }

    #[test]
    fn one_scripted_session_keeps_temporary_state_and_closes_once() {
        let steps = vec![
            ScriptStep {
                call: ScriptCall::Exec(
                    "CREATE TEMPORARY TABLE conformance(value BIGINT)".to_owned(),
                ),
                outcome: ScriptOutcome::Command(command(0, None, 0)),
            },
            ScriptStep {
                call: ScriptCall::Exec("INSERT INTO conformance VALUES (7)".to_owned()),
                outcome: ScriptOutcome::Command(command(1, Some(0), 2)),
            },
            ScriptStep {
                call: ScriptCall::Query("SELECT value FROM conformance".to_owned()),
                outcome: ScriptOutcome::Rows(one_signed(7)),
            },
        ];
        let driver = ScriptedDriver::new(
            steps,
            ResultLimits {
                max_rows: Some(10),
                max_result_bytes: Some(4_096),
            },
            ResultLimits::default(),
        );
        let probe = driver.probe();
        {
            let mut connection = driver
                .connect(options(None, None))
                .expect("connect scripted session");
            assert_eq!(
                connection
                    .exec("CREATE TEMPORARY TABLE conformance(value BIGINT)")
                    .expect("create temporary table"),
                command(0, None, 0)
            );
            assert_eq!(
                connection
                    .exec("INSERT INTO conformance VALUES (7)")
                    .expect("insert temporary row"),
                command(1, Some(0), 2)
            );
            assert_eq!(
                connection
                    .query("SELECT value FROM conformance")
                    .expect("query temporary row"),
                one_signed(7)
            );
            assert_eq!(connection.remaining_steps(), 0);
            connection.close();
            connection.close();
            assert_eq!(
                connection
                    .query("SELECT value FROM conformance")
                    .expect_err("method after close")
                    .class,
                ErrorClass::Closed
            );
        }
        assert_eq!(
            probe.snapshot(),
            SessionSnapshot {
                connections: 1,
                calls: 3,
                close_events: 1,
            }
        );
    }

    #[test]
    fn query_and_exec_reject_the_wrong_result_arm_without_false_success() {
        let driver = ScriptedDriver::new(
            vec![
                ScriptStep {
                    call: ScriptCall::Query("UPDATE fixture".to_owned()),
                    outcome: ScriptOutcome::Command(command(1, None, 0)),
                },
                ScriptStep {
                    call: ScriptCall::Exec("SELECT value".to_owned()),
                    outcome: ScriptOutcome::Rows(one_signed(7)),
                },
            ],
            ResultLimits::default(),
            ResultLimits::default(),
        );
        let mut connection = driver
            .connect(options(None, None))
            .expect("connect scripted session");
        assert_eq!(
            connection
                .query("UPDATE fixture")
                .expect_err("query must not manufacture empty rows")
                .class,
            ErrorClass::Unsupported
        );
        assert_eq!(
            connection
                .exec("SELECT value")
                .expect_err("exec must not discard rows as a success")
                .class,
            ErrorClass::Unsupported
        );
        assert_eq!(connection.remaining_steps(), 0);
    }

    #[test]
    fn terminal_failures_close_without_retry_but_server_errors_can_continue() {
        let server = Error {
            class: ErrorClass::Server,
            vendor_code: Some(1201),
            sqlstate: Some("HY000".to_owned()),
            message: "fixture rejection".to_owned(),
        };
        let driver = ScriptedDriver::new(
            vec![
                ScriptStep {
                    call: ScriptCall::Query("ERROR server".to_owned()),
                    outcome: ScriptOutcome::Error(server.clone()),
                },
                ScriptStep {
                    call: ScriptCall::Query("SELECT value".to_owned()),
                    outcome: ScriptOutcome::Rows(one_signed(7)),
                },
                ScriptStep {
                    call: ScriptCall::Query("ERROR timeout".to_owned()),
                    outcome: ScriptOutcome::Error(error(ErrorClass::Timeout, "fixture timeout")),
                },
            ],
            ResultLimits::default(),
            ResultLimits::default(),
        );
        let probe = driver.probe();
        let mut connection = driver
            .connect(options(None, None))
            .expect("connect scripted session");
        assert_eq!(
            connection.query("ERROR server").expect_err("server error"),
            server
        );
        assert_eq!(
            connection.query("SELECT value").expect("session continues"),
            one_signed(7)
        );
        assert_eq!(
            connection
                .query("ERROR timeout")
                .expect_err("timeout remains distinct")
                .class,
            ErrorClass::Timeout
        );
        assert_eq!(
            connection
                .query("SELECT value")
                .expect_err("terminal timeout closes session")
                .class,
            ErrorClass::Closed
        );
        assert_eq!(connection.remaining_steps(), 0);
        assert_eq!(probe.snapshot().connections, 1);
        assert_eq!(probe.snapshot().calls, 3);
        assert_eq!(probe.snapshot().close_events, 1);
    }

    #[test]
    fn every_terminal_class_closes_and_drop_is_idempotent() {
        for class in [
            ErrorClass::Transport,
            ErrorClass::Protocol,
            ErrorClass::Encoding,
            ErrorClass::Limit,
            ErrorClass::Timeout,
        ] {
            let driver = ScriptedDriver::new(
                vec![ScriptStep {
                    call: ScriptCall::Query("fail".to_owned()),
                    outcome: ScriptOutcome::Error(error(class, "terminal fixture error")),
                }],
                ResultLimits::default(),
                ResultLimits::default(),
            );
            let probe = driver.probe();
            {
                let mut connection = driver
                    .connect(options(None, None))
                    .expect("connect terminal fixture");
                assert_eq!(
                    connection
                        .query("fail")
                        .expect_err("terminal failure")
                        .class,
                    class
                );
                assert_eq!(
                    connection
                        .query("fail")
                        .expect_err("terminal failure closes resource")
                        .class,
                    ErrorClass::Closed
                );
            }
            assert_eq!(probe.snapshot().connections, 1);
            assert_eq!(probe.snapshot().calls, 1);
            assert_eq!(probe.snapshot().close_events, 1);
        }

        let driver =
            ScriptedDriver::new(Vec::new(), ResultLimits::default(), ResultLimits::default());
        let probe = driver.probe();
        drop(
            driver
                .connect(options(None, None))
                .expect("connection dropped without explicit close"),
        );
        assert_eq!(probe.snapshot().close_events, 1);
    }

    #[test]
    fn invalid_connect_and_bad_call_order_fail_explicitly() {
        let driver = ScriptedDriver::new(
            vec![ScriptStep {
                call: ScriptCall::Exec("expected".to_owned()),
                outcome: ScriptOutcome::Command(command(0, None, 0)),
            }],
            ResultLimits::default(),
            ResultLimits::default(),
        );
        let mut invalid = options(None, None);
        invalid.endpoint.clear();
        assert_eq!(
            driver
                .connect(invalid)
                .expect_err("empty endpoint must be invalid")
                .class,
            ErrorClass::Invalid
        );
        assert_eq!(driver.probe().snapshot().connections, 0);

        let mut connection = driver.connect(options(None, None)).expect("valid connect");
        assert_eq!(
            connection
                .query("unexpected")
                .expect_err("bad call order must be protocol")
                .class,
            ErrorClass::Protocol
        );
        assert_eq!(
            connection
                .exec("expected")
                .expect_err("protocol failure closes session")
                .class,
            ErrorClass::Closed
        );
    }

    #[test]
    fn positional_metadata_preserves_empty_and_duplicate_labels() {
        let row_set = RowSet {
            columns: vec![
                column("", ColumnType::Signed, None),
                column("duplicate", ColumnType::Decimal, None),
                column(
                    "duplicate",
                    ColumnType::Temporal,
                    Some(TemporalType::Timestamp),
                ),
            ],
            rows: vec![Row {
                cells: vec![
                    Cell::Signed(7),
                    Cell::Decimal("7.00".to_owned()),
                    Cell::Temporal("1970-01-01 00:00:07".to_owned()),
                ],
            }],
        };
        assert_eq!(row_set.columns[0].name, "");
        assert_eq!(row_set.columns[1].name, row_set.columns[2].name);
        assert!(logical_row_set_bytes(&row_set).is_ok());
    }
}
