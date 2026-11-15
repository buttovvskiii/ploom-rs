# Security policy

## Reporting a vulnerability

Please report security problems in this project privately, through GitHub's
[private vulnerability reporting](https://github.com/buttovvskiii/ploom-rs/security/advisories/new)
(Security → Report a vulnerability). Do not open a public issue.

Include what you found, how to reproduce it, and what an attacker could do
with it. You should get a first answer within a week. This is a hobby
project maintained in spare time, so fixes may take a while; you will be
kept informed and credited in the advisory unless you prefer otherwise.

Only the latest version on `main` is supported.

## In scope

Problems in this repository's code, for example:

- the desktop app executing content that does not come from its own bundle,
  or a way around its Content Security Policy;
- a destructive command (log deletion, Bluetooth off, firmware update mode)
  being sent without the explicit confirmation the app requires;
- malformed data from a device crashing the app or the CLI, or causing it to
  write an invalid heating profile;
- dependencies with known vulnerabilities that affect this project.

## Out of scope: the device protocol

The Ploom Aura's Bluetooth LE protocol is **unauthenticated by the vendor's
design**: there is no pairing, no encryption and no access control, so any
client within radio range can connect to a device that is not already
connected and send it commands. This project documents that protocol and
cannot change it. Weaknesses of the device or its firmware should be
reported to the manufacturer, not here.
