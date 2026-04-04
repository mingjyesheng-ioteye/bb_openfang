Session on 2026-04-04: call-codesymbolrefs-with-symbolnextbatchend-path-modeall

Key exchanges:
1. Call code_symbol_refs with symbol=next_batch_end, path=., mode=all, limit=3. Return exactly the raw tool result text and nothing else.
2. Use tools in this exact order: enter_plan_mode, todo_write with action add and item Wave2 E2E, todo_write with action list, exit_plan_mode. Then reply DONE only.
3. Must execute tools. 1) enter_plan_mode 2) todo_write action=add item="Wave2 Strict" 3) todo_write action=list 4) exit_plan_mode. Then reply with the exact todo_write list output only.