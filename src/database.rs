// Currently this is unused, and will stay so until the model and api are finished and I can transition from the PasteManager's mock storage to an actual database
use rusqlite::Connection;
pub struct Database {}
impl Database {
    pub fn init() -> Connection {
        let conn = Connection::open("main.db").unwrap();
        let table_result = conn.execute(
            "create table if not exists pastes (
                 id             integer primary key,
                 url            text,
                 password       text,
                 content        text,
                 date_published text,
                 date_edited    text
             )", ());
        match table_result {
            Ok(_)  => println!("Successfully connected to table."),
            Err(e) => panic!("Database creation failed with error message: {e}")
        };
        conn
    }
}