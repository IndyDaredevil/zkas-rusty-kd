## HANDOFF MAILBOX

Lane id: zkas-node. Supabase project drfqooqkicdhhwkjjaez
(bolt-native-database-52370489), via the Supabase MCP connector (deferred
tool; load with tool_search "Supabase execute_sql"). Mailbox:
public.project_handoffs. Convention: public.handoff_docs, name
HANDOFF-CONVENTION-r3 (or latest r*).

Before writing any row: read the convention AND verify its
db_sha256 == pinned_sha256 (convention rule 4) — then write.

Query the mailbox only on turns Michael opens with "check the mailbox".
Never guess the schema; read the convention.

This is the same database the mining dashboard reads. From this lane, write
only public.project_handoffs and public.handoff_docs; never write the
dashboard's application tables (zkas_blocks, network_history, restarts,
miners, etc.) except as an explicit, handoff-authorized task.
