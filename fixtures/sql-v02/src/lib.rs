#![deny(unsafe_code)]

use std::cell::RefCell;

#[allow(unsafe_code, clippy::all, clippy::nursery, clippy::pedantic)]
mod bindings {
    wit_bindgen::generate!({
        path: "wit",
        world: "fixture",
        generate_all,
    });
}

use bindings::exports::sigil::sql::driver::{
    Cell as SqlCell, Column, ColumnType, CommandResult, ConnectOptions, Connection, Error,
    ErrorClass, Guest, GuestConnection, Row, RowSet, TemporalType,
};

struct Fixture;

#[derive(Default)]
struct State {
    closed: bool,
    temporary: Vec<i64>,
}

struct FixtureConnection {
    state: RefCell<State>,
    max_rows: Option<u32>,
    max_result_bytes: Option<u64>,
}

fn failure(class: ErrorClass, message: &str) -> Error {
    Error {
        class,
        vendor_code: None,
        sqlstate: None,
        message: message.to_owned(),
    }
}

fn terminal(connection: &FixtureConnection, class: ErrorClass, message: &str) -> Error {
    connection.state.borrow_mut().closed = true;
    failure(class, message)
}

impl Guest for Fixture {
    type Connection = FixtureConnection;

    fn connect(options: ConnectOptions) -> Result<Connection, Error> {
        if options.endpoint.is_empty()
            || options.username_secret.is_empty()
            || options.password_secret.is_empty()
        {
            return Err(failure(
                ErrorClass::Invalid,
                "endpoint and secret names must not be empty",
            ));
        }
        Ok(Connection::new(FixtureConnection {
            state: RefCell::new(State::default()),
            max_rows: options.max_rows,
            max_result_bytes: options.max_result_bytes,
        }))
    }
}

impl GuestConnection for FixtureConnection {
    #[inline(never)]
    fn query(&self, sql: String) -> Result<RowSet, Error> {
        if self.state.borrow().closed {
            return Err(failure(ErrorClass::Closed, "connection is closed"));
        }
        let rows = match sql.as_str() {
            "SELECT typed" => typed_rows(),
            "SELECT value FROM conformance" => temporary_rows(&self.state.borrow().temporary),
            "COMMAND" => {
                return Err(failure(
                    ErrorClass::Unsupported,
                    "query received a command response",
                ));
            }
            "ERROR server" => {
                return Err(Error {
                    class: ErrorClass::Server,
                    vendor_code: Some(1201),
                    sqlstate: Some("HY000".to_owned()),
                    message: "fixture rejection".to_owned(),
                });
            }
            "ERROR timeout" => {
                return Err(terminal(self, ErrorClass::Timeout, "fixture timeout"));
            }
            "ERROR transport" => {
                return Err(terminal(
                    self,
                    ErrorClass::Transport,
                    "fixture transport failure",
                ));
            }
            _ => {
                return Err(failure(
                    ErrorClass::Unsupported,
                    "statement is outside the conformance fixture",
                ));
            }
        };
        self.admit_rows(rows)
    }

    #[inline(never)]
    fn exec(&self, sql: String) -> Result<CommandResult, Error> {
        if self.state.borrow().closed {
            return Err(failure(ErrorClass::Closed, "connection is closed"));
        }
        let command = match sql.as_str() {
            "CREATE TEMPORARY TABLE conformance(value BIGINT)" => {
                self.state.borrow_mut().temporary.clear();
                CommandResult {
                    affected_rows: 0,
                    last_insert_id: None,
                    warnings: 0,
                }
            }
            "INSERT INTO conformance VALUES (7)" => {
                self.state.borrow_mut().temporary.push(7);
                CommandResult {
                    affected_rows: 1,
                    last_insert_id: Some(0),
                    warnings: 2,
                }
            }
            "SELECT typed" => {
                return Err(failure(
                    ErrorClass::Unsupported,
                    "exec received a row response",
                ));
            }
            "ERROR server" => {
                return Err(Error {
                    class: ErrorClass::Server,
                    vendor_code: Some(1201),
                    sqlstate: Some("HY000".to_owned()),
                    message: "fixture rejection".to_owned(),
                });
            }
            _ => {
                return Err(failure(
                    ErrorClass::Unsupported,
                    "statement is outside the conformance fixture",
                ));
            }
        };
        self.admit_command(command)
    }

    fn close(&self) {
        self.state.borrow_mut().closed = true;
    }
}

impl FixtureConnection {
    #[inline(never)]
    fn admit_rows(&self, rows: RowSet) -> Result<RowSet, Error> {
        let row_count = u32::try_from(rows.rows.len())
            .map_err(|_error| terminal(self, ErrorClass::Limit, "fixture row limit"))?;
        if self.max_rows.is_some_and(|maximum| row_count > maximum) {
            return Err(terminal(self, ErrorClass::Limit, "fixture row limit"));
        }
        let bytes = logical_row_set_bytes(&rows)
            .ok_or_else(|| terminal(self, ErrorClass::Limit, "fixture byte overflow"))?;
        if self.max_result_bytes.is_some_and(|maximum| bytes > maximum) {
            return Err(terminal(self, ErrorClass::Limit, "fixture byte limit"));
        }
        Ok(rows)
    }

    #[inline(never)]
    fn admit_command(&self, command: CommandResult) -> Result<CommandResult, Error> {
        let bytes = if command.last_insert_id.is_some() {
            20
        } else {
            12
        };
        if self.max_result_bytes.is_some_and(|maximum| bytes > maximum) {
            return Err(terminal(self, ErrorClass::Limit, "fixture byte limit"));
        }
        Ok(command)
    }
}

#[inline(never)]
fn typed_rows() -> RowSet {
    RowSet {
        columns: vec![
            column("null", 6, ColumnType::Null, None),
            column("signed", 8, ColumnType::Signed, None),
            column("unsigned", 8, ColumnType::Unsigned, None),
            column("floating", 5, ColumnType::Floating, None),
            column("decimal", 246, ColumnType::Decimal, None),
            column("text", 253, ColumnType::Text, None),
            column("bytes", 252, ColumnType::Bytes, None),
            column(
                "temporal",
                7,
                ColumnType::Temporal,
                Some(TemporalType::TimestampWithTimeZone),
            ),
        ],
        rows: vec![Row {
            cells: vec![
                SqlCell::Null,
                SqlCell::Signed(i64::MIN),
                SqlCell::Unsigned(u64::MAX),
                SqlCell::Floating(-0.0),
                SqlCell::Decimal("001.2300".to_owned()),
                SqlCell::Text("snowman ☃".to_owned()),
                SqlCell::Bytes(vec![0, 255, 128]),
                SqlCell::Temporal("2026-08-30 12:34:56.000001+05:45".to_owned()),
            ],
        }],
    }
}

#[inline(never)]
fn temporary_rows(values: &[i64]) -> RowSet {
    RowSet {
        columns: vec![column("value", 8, ColumnType::Signed, None)],
        rows: values
            .iter()
            .map(|value| Row {
                cells: vec![SqlCell::Signed(*value)],
            })
            .collect(),
    }
}

#[inline(never)]
fn column(
    name: &str,
    vendor_type: u32,
    column_type: ColumnType,
    temporal_type: Option<TemporalType>,
) -> Column {
    Column {
        catalog: String::new(),
        schema: String::new(),
        table: "fixture".to_owned(),
        original_table: "fixture".to_owned(),
        name: name.to_owned(),
        original_name: name.to_owned(),
        vendor_type,
        charset: 0,
        collation: 0,
        flags: 0,
        type_: column_type,
        temporal_type,
    }
}

#[inline(never)]
fn logical_row_set_bytes(rows: &RowSet) -> Option<u64> {
    let metadata = rows.columns.iter().try_fold(0_u64, |total, column| {
        [
            &column.catalog,
            &column.schema,
            &column.table,
            &column.original_table,
            &column.name,
            &column.original_name,
        ]
        .into_iter()
        .try_fold(total, |subtotal, value| {
            subtotal.checked_add(u64::try_from(value.len()).ok()?)
        })
    })?;
    rows.rows.iter().try_fold(metadata, |total, row| {
        row.cells.iter().try_fold(total, |subtotal, cell| {
            let bytes = match cell {
                SqlCell::Null => 0,
                SqlCell::Signed(_) | SqlCell::Unsigned(_) | SqlCell::Floating(_) => 8,
                SqlCell::Decimal(value) | SqlCell::Text(value) | SqlCell::Temporal(value) => {
                    u64::try_from(value.len()).ok()?
                }
                SqlCell::Bytes(value) => u64::try_from(value.len()).ok()?,
            };
            subtotal.checked_add(bytes)
        })
    })
}

#[allow(unsafe_code, clippy::all, clippy::nursery, clippy::pedantic)]
#[cfg(target_arch = "wasm32")]
mod export {
    use super::Fixture;

    crate::bindings::export!(Fixture with_types_in crate::bindings);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connection(max_rows: Option<u32>, max_result_bytes: Option<u64>) -> FixtureConnection {
        FixtureConnection {
            state: RefCell::new(State::default()),
            max_rows,
            max_result_bytes,
        }
    }

    #[test]
    fn typed_fixture_preserves_every_value_and_command_field() {
        let connection = connection(None, None);
        let rows =
            <FixtureConnection as GuestConnection>::query(&connection, "SELECT typed".to_owned())
                .expect("typed rows");
        assert_eq!(rows.columns.len(), 8);
        assert_eq!(rows.rows[0].cells.len(), 8);
        assert!(matches!(rows.rows[0].cells[0], SqlCell::Null));
        assert!(matches!(rows.rows[0].cells[1], SqlCell::Signed(i64::MIN)));
        assert!(matches!(rows.rows[0].cells[2], SqlCell::Unsigned(u64::MAX)));
        assert!(
            matches!(rows.rows[0].cells[3], SqlCell::Floating(value) if value.to_bits() == (-0.0_f64).to_bits())
        );
        assert!(matches!(&rows.rows[0].cells[4], SqlCell::Decimal(value) if value == "001.2300"));
        assert!(
            matches!(&rows.rows[0].cells[7], SqlCell::Temporal(value) if value == "2026-08-30 12:34:56.000001+05:45")
        );
        assert!(matches!(
            rows.columns[7].temporal_type,
            Some(TemporalType::TimestampWithTimeZone)
        ));

        let create = <FixtureConnection as GuestConnection>::exec(
            &connection,
            "CREATE TEMPORARY TABLE conformance(value BIGINT)".to_owned(),
        )
        .expect("create temporary table");
        assert_eq!(create.affected_rows, 0);
        assert_eq!(create.last_insert_id, None);
        assert_eq!(create.warnings, 0);
        let insert = <FixtureConnection as GuestConnection>::exec(
            &connection,
            "INSERT INTO conformance VALUES (7)".to_owned(),
        )
        .expect("insert temporary row");
        assert_eq!(insert.affected_rows, 1);
        assert_eq!(insert.last_insert_id, Some(0));
        assert_eq!(insert.warnings, 2);
        let temporary = <FixtureConnection as GuestConnection>::query(
            &connection,
            "SELECT value FROM conformance".to_owned(),
        )
        .expect("same-session temporary row");
        assert!(matches!(temporary.rows[0].cells[0], SqlCell::Signed(7)));
    }

    #[test]
    fn fixture_separates_query_exec_errors_and_discards_limited_results() {
        let open = connection(None, None);
        assert!(matches!(
            <FixtureConnection as GuestConnection>::query(&open, "COMMAND".to_owned()),
            Err(Error {
                class: ErrorClass::Unsupported,
                ..
            })
        ));
        assert!(matches!(
            <FixtureConnection as GuestConnection>::exec(&open, "SELECT typed".to_owned()),
            Err(Error {
                class: ErrorClass::Unsupported,
                ..
            })
        ));
        let server =
            <FixtureConnection as GuestConnection>::query(&open, "ERROR server".to_owned())
                .expect_err("server error");
        assert_eq!(server.class, ErrorClass::Server);
        assert_eq!(server.vendor_code, Some(1201));
        assert_eq!(server.sqlstate.as_deref(), Some("HY000"));

        let limited = connection(Some(0), None);
        assert!(matches!(
            <FixtureConnection as GuestConnection>::query(&limited, "SELECT typed".to_owned()),
            Err(Error {
                class: ErrorClass::Limit,
                ..
            })
        ));
        assert!(matches!(
            <FixtureConnection as GuestConnection>::query(&limited, "SELECT typed".to_owned()),
            Err(Error {
                class: ErrorClass::Closed,
                ..
            })
        ));

        let command_limited = connection(None, Some(19));
        <FixtureConnection as GuestConnection>::exec(
            &command_limited,
            "CREATE TEMPORARY TABLE conformance(value BIGINT)".to_owned(),
        )
        .expect("twelve-byte command");
        assert!(matches!(
            <FixtureConnection as GuestConnection>::exec(
                &command_limited,
                "INSERT INTO conformance VALUES (7)".to_owned()
            ),
            Err(Error {
                class: ErrorClass::Limit,
                ..
            })
        ));
    }

    #[test]
    fn timeout_and_transport_remain_distinct_terminal_classes() {
        for (statement, class) in [
            ("ERROR timeout", ErrorClass::Timeout),
            ("ERROR transport", ErrorClass::Transport),
        ] {
            let connection = connection(None, None);
            assert!(matches!(
                <FixtureConnection as GuestConnection>::query(&connection, statement.to_owned()),
                Err(Error { class: actual, .. }) if actual == class
            ));
            assert!(matches!(
                <FixtureConnection as GuestConnection>::query(
                    &connection,
                    "SELECT typed".to_owned()
                ),
                Err(Error {
                    class: ErrorClass::Closed,
                    ..
                })
            ));
        }
    }
}
