"""The repository's automation support code, driven by `dodo.py`.

The implementations live in `scripts/`, which is where they already are: moving eighteen
modules and twenty-six test files would be churn with no caller asking for it. What this
package holds is the part that is new — the bridge between doit's task model and a script's
`main()`, and the facts about the repository that several tasks need to agree on.

`dodo.py` stays a registry: it names tasks, their actions and their order, and nothing else.
"""
