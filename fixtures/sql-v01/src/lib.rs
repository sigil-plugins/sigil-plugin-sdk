#![deny(unsafe_code)]

use std::cell::Cell;

#[allow(unsafe_code, clippy::all, clippy::nursery, clippy::pedantic)]
mod bindings {
    wit_bindgen::generate!({
        path: "wit",
        world: "fixture",
        generate_all,
    });
}

use bindings::exports::sigil::sql::driver::{
    Cell as SqlCell, Column, CommandResult, ConnectOptions, Connection, Error, ErrorClass, Guest,
    GuestConnection, QueryResult, Row, RowSet,
};

struct Fixture;

struct FixtureConnection {
    closed: Cell<bool>,
}

fn failure(class: ErrorClass, message: &str) -> Error {
    Error {
        class,
        vendor_code: None,
        sqlstate: None,
        message: message.to_owned(),
    }
}

impl Guest for Fixture {
    type Connection = FixtureConnection;

    fn connect(_options: ConnectOptions) -> Result<Connection, Error> {
        Ok(Connection::new(FixtureConnection {
            closed: Cell::new(false),
        }))
    }
}

impl GuestConnection for FixtureConnection {
    fn query(&self, sql: String) -> Result<QueryResult, Error> {
        if self.closed.get() {
            return Err(failure(ErrorClass::Closed, "connection is closed"));
        }
        match sql.as_str() {
            "SELECT lossless" => Ok(QueryResult::Rows(RowSet {
                columns: vec![column("null", 6), column("text", 253), column("bytes", 252)],
                rows: vec![Row {
                    cells: vec![
                        SqlCell::Null,
                        SqlCell::Text("snowman ☃".to_owned()),
                        SqlCell::Bytes(vec![0, 255, 128]),
                    ],
                }],
            })),
            "UPDATE fixture" => Ok(QueryResult::Command(CommandResult { affected_rows: 1 })),
            _ => Err(failure(
                ErrorClass::Unsupported,
                "statement is outside the conformance fixture",
            )),
        }
    }

    fn close(&self) {
        self.closed.set(true);
    }
}

fn column(name: &str, vendor_type: u32) -> Column {
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
    }
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

    #[test]
    fn frozen_fixture_keeps_combined_query_result_and_idempotent_close() {
        let connection = FixtureConnection {
            closed: Cell::new(false),
        };
        let rows = <FixtureConnection as GuestConnection>::query(
            &connection,
            "SELECT lossless".to_owned(),
        )
        .expect("lossless rows");
        let QueryResult::Rows(rows) = rows else {
            panic!("frozen query fixture changed result arm");
        };
        assert!(matches!(rows.rows[0].cells[0], SqlCell::Null));
        assert!(matches!(&rows.rows[0].cells[1], SqlCell::Text(value) if value == "snowman ☃"));
        assert!(matches!(&rows.rows[0].cells[2], SqlCell::Bytes(value) if value == &[0, 255, 128]));

        let command =
            <FixtureConnection as GuestConnection>::query(&connection, "UPDATE fixture".to_owned())
                .expect("combined command arm");
        assert!(matches!(
            command,
            QueryResult::Command(CommandResult { affected_rows: 1 })
        ));
        <FixtureConnection as GuestConnection>::close(&connection);
        <FixtureConnection as GuestConnection>::close(&connection);
        assert!(matches!(
            <FixtureConnection as GuestConnection>::query(
                &connection,
                "SELECT lossless".to_owned()
            ),
            Err(Error {
                class: ErrorClass::Closed,
                ..
            })
        ));
    }
}
