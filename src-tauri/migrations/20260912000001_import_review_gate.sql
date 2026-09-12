-- Typed review reasons are durable so acknowledgement stays tied to the exact
-- analysis revision that produced the preview.
ALTER TABLE import_jobs ADD COLUMN review_gate_json TEXT NOT NULL DEFAULT '{"reasons":[]}'
    CHECK(json_valid(review_gate_json));
