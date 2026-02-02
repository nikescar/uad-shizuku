-- Add file_path column to virustotal_results table
-- ALTER TABLE virustotal_results ADD COLUMN file_path TEXT NOT NULL DEFAULT '';

-- Create a new table with the correct schema including the UNIQUE constraint
CREATE TABLE virustotal_results_new (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  package_name TEXT NOT NULL,
  file_path TEXT NOT NULL,
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
  updated_at INTEGER NOT NULL,
  UNIQUE(package_name, file_path, sha256)
);

-- Copy data from old table to new table
INSERT INTO virustotal_results_new (
  id, package_name, file_path, sha256, last_analysis_date,
  malicious, suspicious, undetected, harmless, timeout,
  failure, type_unsupported, dex_count, reputation,
  raw_response, created_at, updated_at
)
SELECT
  id, package_name, file_path, sha256, last_analysis_date,
  malicious, suspicious, undetected, harmless, timeout,
  failure, type_unsupported, dex_count, reputation,
  raw_response, created_at, updated_at
FROM virustotal_results;

-- Drop old table
DROP TABLE virustotal_results;

-- Rename new table to original name
ALTER TABLE virustotal_results_new RENAME TO virustotal_results;
