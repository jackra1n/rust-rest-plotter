CREATE TABLE IF NOT EXISTS DefaultTests (
  id SERIAL PRIMARY KEY,
  name VARCHAR(255),
  branch VARCHAR(255),
  build_number BIGINT,
  runtime BIGINT,
  UNIQUE (name, branch, build_number)
)