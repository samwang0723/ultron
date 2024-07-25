# Repository database connections
Using sqlx requires runtime validation through real database.
Thus, have to configure a user for sqlx to check.

```sql
CREATE USER samwang;
GRANT ALL PRIVILEGES ON DATABASE jarvis_main TO samwang;
```
