wit_bindgen::generate!({
    path: "../wit",
    world: "app",
});

use crate::demo::db::database;

use crate::exports::demo::db::application::Guest;

struct Application;

impl Guest for Application {
    fn run() -> String {
        match database::execute(
            "create table if not exists people (
    id integer primary key,
    name text
)",
        ) {
            Ok(_) => {}
            Err(error) => return format!("create table failed: {error}"),
        }

        match database::execute("insert or ignore into people (id, name) values (1, 'Ada')") {
            Ok(_) => {}
            Err(error) => return format!("insert failed: {error}"),
        }

        match database::query("select id, name from people") {
            Ok(result) => {
                let mut output = String::new();

                for row in result.rows {
                    output.push_str(&row.join(" | "));
                    output.push('\n');
                }

                output
            }
            Err(error) => format!("query failed: {error}"),
        }
    }
}

export!(Application);
