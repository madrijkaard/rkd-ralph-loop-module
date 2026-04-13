CREATE TABLE project (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50),
    created_date TIMESTAMP,
    last_modified_date TIMESTAMP,
    status VARCHAR(20)
);

CREATE TABLE use_case (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50),
    prompt TEXT,
    created_date TIMESTAMP,
    last_modified_date TIMESTAMP,
    status VARCHAR(20),
    project_id INTEGER REFERENCES project(id)
);

CREATE TABLE task (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50),
    sequence INTEGER,
    type VARCHAR(50),
    path TEXT,
    prompt TEXT,
    created_date TIMESTAMP,
    last_modified_date TIMESTAMP,
    status VARCHAR(20),
    use_case_id INTEGER REFERENCES use_case(id)
);

CREATE TABLE iteration (
    id SERIAL PRIMARY KEY,
    created_date TIMESTAMP,
    last_modified_date TIMESTAMP,
    status VARCHAR(20),
    task_id INTEGER REFERENCES task(id)
);