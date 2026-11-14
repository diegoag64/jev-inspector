# Security policy

## Report a vulnerability

Don't report security problems in public issues. Report them privately with [GitHub private vulnerability reporting](https://github.com/diegoag64/jev-inspector/security/advisories/new).

Include the affected commit, steps to reproduce, and the impact you observed. You'll get an acknowledgement when the maintainer has read the report. This is a spare-time project, so there is no guaranteed response time.

## Supported versions

Only the latest commit on `main` receives fixes.

## Scope

Jev Inspector is a local development tool. `jev-bridge` forwards your backend's TypeSafe requests, including its API key, and keeps captured requests in memory. The relay and its capture API are meant to listen on loopback only.

These reports are in scope:

- A web page, another user account, or a network host that can read captured traffic through the capture API without its connection token.
- The relay accepting a non-loopback listen address, or forwarding to a non-HTTPS upstream other than a loopback mock.
- API keys or other credentials appearing in captures, logs, exports, or saved sessions.
- A crafted response or saved session that makes the inspector read or write files the user didn't choose.

Problems that require an attacker who already controls your machine, your backend, or the upstream API are out of scope. Field-name redaction is best effort; application data inside free-form text is expected to remain visible.
