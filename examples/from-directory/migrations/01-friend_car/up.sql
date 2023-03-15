CREATE TABLE friend(
    friend_id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT UNIQUE,
    phone TEXT UNIQUE,
    picture BLOB
);

CREATE TABLE car(
    registration_plate TEXT PRIMARY KEY,
    cost REAL NOT NULL,
    bought_on TEXT NOT NULL
);
