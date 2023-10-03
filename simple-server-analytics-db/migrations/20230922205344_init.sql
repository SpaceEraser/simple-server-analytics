CREATE TABLE "sa_connection" (
    "id" BLOB NOT NULL PRIMARY KEY,
    "created_at" DATETIME NOT NULL,
    "local_addr" TEXT NOT NULL,
    "remote_addr" TEXT NOT NULL,
    "http_scheme" TEXT NOT NULL,
    "http_version" TEXT NOT NULL
);

CREATE TABLE "sa_request" (
    "id" BLOB NOT NULL PRIMARY KEY,
    "created_at" DATETIME NOT NULL,
    "conn_id" BLOB NULL REFERENCES "sa_connection" ("id") ON DELETE SET NULL,
    "method" TEXT NOT NULL,
    "path" TEXT NOT NULL,
    "hostname" TEXT NOT NULL,
    "user_agent" TEXT NOT NULL
);

CREATE TABLE "sa_response" (
    "id" BLOB NOT NULL PRIMARY KEY,
    "created_at" DATETIME NOT NULL,
    "conn_id" BLOB NULL REFERENCES "sa_connection" ("id") ON DELETE SET NULL,
    "req_id" BLOB NOT NULL REFERENCES "sa_request" ("id") ON DELETE SET NULL,
    "duration" TEXT NOT NULl,
    "status" INT NOT NULL
);