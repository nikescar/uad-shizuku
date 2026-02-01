-- Revert the migration by removing file_path column
CREATE TABLE virustotal_results_old (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  package_name TEXT NOT NULL,
  sha256 TEXT NOT NULL,
  last_analysis_date INTEGER NOT NULL,
  malicious INTEGER NOT NULL,
  suspicious INTEGER NOT NULL,
  undetected INTEGER NOT NULL,
  harmless INTEGER NOT NULL,
  timeout INTEGER NOT NULL,
  failure INTEGER NOT NULL,
  type_unsupported INTEGER NOT NULL,
  dex_count INTEGER,
  reputation INTEGER NOT NULL,
  raw_response TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

-- Copy data back (file_path will be lost)
INSERT INTO virustotal_results_old (
  id, package_name, sha256, last_analysis_date,
  malicious, suspicious, undetected, harmless, timeout,
  failure, type_unsupported, dex_count, reputation,
  raw_response, created_at, updated_at
)
SELECT
  id, package_name, sha256, last_analysis_date,
  malicious, suspicious, undetected, harmless, timeout,
  failure, type_unsupported, dex_count, reputation,
  raw_response, created_at, updated_at
FROM virustotal_results;

DROP TABLE virustotal_results;
ALTER TABLE virustotal_results_old RENAME TO virustotal_results;
