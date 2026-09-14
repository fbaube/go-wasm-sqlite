package main

import (
"fmt"

"wit_component/gen/demo/db/application"
"wit_component/gen/demo/db/database"
)

func init() {
application.Exports.Run = func() string {
create := database.Execute(
"create table if not exists people (id integer primary key, name text)",
)
if create.IsErr() {
return fmt.Sprintf("create failed: %s", *create.Err())
}

insert := database.Execute(
"insert or ignore into people (id, name) values (1, 'Ada')",
)
if insert.IsErr() {
return fmt.Sprintf("insert failed: %s", *insert.Err())
}

query := database.Query(
"select id, name from people order by id",
)
if query.IsErr() {
return fmt.Sprintf("query failed: %s", *query.Err())
}

result := query.OK()
output := ""

for i, column := range result.Columns.Slice() {
if i > 0 {
output += " | "
}
output += column
}
output += "\n----------------\n"

for _, row := range result.Rows.Slice() {
for i, value := range row.Slice() {
if i > 0 {
output += " | "
}
output += value
}
output += "\n"
}

return output
}
}

func main() {}
