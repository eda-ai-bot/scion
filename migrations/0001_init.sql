-- Scion metadata schema, v1.
--
-- Everything here is metadata. File contents and trees live in the object store
-- (see docs/DATA_MODEL.md). Every table carries a `repo` column: one server hosts
-- many repositories from day one.

CREATE TABLE repos (
    id          BIGSERIAL PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- The write-ahead log. One row per changeset; immutable once written.
-- `id` is blake3(canonical(changeset)) — see docs/DATA_MODEL.md.
CREATE TABLE changesets (
    repo            BIGINT NOT NULL REFERENCES repos(id),
    id              BYTEA  NOT NULL CHECK (octet_length(id) = 32),
    parents         BYTEA[] NOT NULL,
    first_parent    BYTEA  CHECK (first_parent IS NULL OR octet_length(first_parent) = 32),
    author_name     TEXT   NOT NULL,
    author_email    TEXT   NOT NULL,
    author_date     BIGINT NOT NULL,          -- unix seconds
    author_tz       INTEGER NOT NULL,         -- offset from UTC in minutes
    committer_name  TEXT,
    committer_email TEXT,
    committer_date  BIGINT,
    committer_tz    INTEGER,
    message         TEXT   NOT NULL,
    signature       BYTEA,                    -- detached; NOT part of the hash
    generation      BIGINT NOT NULL,          -- max(parent.generation) + 1; roots are 0
    canonical       BYTEA  NOT NULL,          -- canonical encoding; id = blake3(canonical)
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (repo, id)
);
CREATE INDEX changesets_first_parent ON changesets (repo, first_parent);
CREATE INDEX changesets_generation   ON changesets (repo, generation);

-- Files changed relative to the FIRST parent only (trunk-based history).
CREATE TABLE file_changes (
    repo            BIGINT NOT NULL,
    changeset_id    BYTEA  NOT NULL,
    path            TEXT   NOT NULL,
    change_kind     SMALLINT NOT NULL,        -- 0 = deleted, 1 = tracked
    blob_id         BYTEA  CHECK (blob_id IS NULL OR octet_length(blob_id) = 32),
    file_type       SMALLINT,                 -- 0 = regular, 1 = executable, 2 = symlink
    size            BIGINT,
    copy_from_path  TEXT,
    copy_from_cs    BYTEA,
    PRIMARY KEY (repo, changeset_id, path),
    FOREIGN KEY (repo, changeset_id) REFERENCES changesets(repo, id)
);
CREATE INDEX file_changes_path ON file_changes (repo, path);

-- Refs: ONLY `main` and `release/*` are allowed server-side. Moving a ref is a
-- single-row compare-and-swap on `version`.
CREATE TABLE refs (
    repo        BIGINT NOT NULL REFERENCES repos(id),
    name        TEXT   NOT NULL,
    target      BYTEA  NOT NULL CHECK (octet_length(target) = 32),
    version     BIGINT NOT NULL DEFAULT 0,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (repo, name),
    CHECK (name = 'main' OR name LIKE 'release/%')
);

-- Generic derived-data pointer table. For `root_tree` the pointer is the TreeId
-- (an object-store key); other types point at their own object-store blobs.
CREATE TABLE derived (
    repo            BIGINT NOT NULL,
    type            TEXT   NOT NULL,
    version         INTEGER NOT NULL,
    changeset_id    BYTEA  NOT NULL,
    pointer         BYTEA  NOT NULL,
    derived_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (repo, type, version, changeset_id)
);

CREATE TYPE change_request_status AS ENUM ('pending', 'landing', 'landed', 'needs_rebase', 'rejected');

CREATE TABLE change_requests (
    repo          BIGINT NOT NULL REFERENCES repos(id),
    id            BIGSERIAL,
    head_cs       BYTEA NOT NULL,
    base_cs       BYTEA NOT NULL,
    status        change_request_status NOT NULL DEFAULT 'pending',
    landed_cs     BYTEA,
    conflicts     TEXT[],
    submitted_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    landed_at     TIMESTAMPTZ,
    PRIMARY KEY (repo, id)
);
CREATE INDEX change_requests_pending ON change_requests (repo, submitted_at) WHERE status = 'pending';

-- Resumable git ingestion.
CREATE TABLE ingest_checkpoints (
    repo          BIGINT NOT NULL REFERENCES repos(id),
    git_sha       BYTEA  NOT NULL CHECK (octet_length(git_sha) IN (20, 32)),
    changeset_id  BYTEA  NOT NULL,
    PRIMARY KEY (repo, git_sha)
);
