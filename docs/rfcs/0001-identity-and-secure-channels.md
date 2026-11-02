# RFC 0001: Identity and secure channels as small schemes

| | |
|---|---|
| Status | Draft |
| Created | 2026-11-02 |
| Issue | [#180](https://github.com/redox-rpi/redox-rpi/issues/180) |
| Releases | v0.6.0 (first steps), v0.7.0, v0.8.0, later |

## Summary

Encryption and identity become part of the system, not of each program.
A few small daemons, each offering one Redox scheme, provide key storage,
encrypted connections, trust, accounts and consent. Programs open
`secure:` instead of `tcp:` and get an encrypted, authenticated channel
and the verified identity of the other side. They never see a private key.

## Motivation

Today every protocol solves three problems on its own:

1. **Channel:** nobody can read or change the data (TLS, SSH, VPN, E2E chat).
2. **Identity:** who is on the other end, and who am I (certificates, SSH
   keys, passwords, password managers).
3. **Sharing data:** what a service gets from me (name, address, payment).

Each program brings its own crypto library, its own key files and its own
way to log in. Users set up the same thing again and again, and often
leave encryption off. On a small system like the Pi, every program that
links its own TLS stack also costs memory and build time.

Redox has a good base for doing this once: every resource is a scheme
served by a user-space daemon, daemons can stack on each other, and each
process has a namespace that decides which schemes it can see.

## Goals

- Connections between systems that support this are encrypted and
  authenticated by default. Plain only when asked for.
- Registering with a service is one step: scan a code, confirm.
- Private keys stay inside one daemon. Programs get the right to use a
  key, not the key.
- One user works on all of the user's machines.
- Each part is a small daemon that can be used, tested and replaced alone.

## Non-goals

- Replacing TLS and SSH for the open internet. Those stay as the fallback
  for peers that don't support this.
- A global naming or certificate system. Names are bound to keys locally
  (pinning, codes, trust on first use); public certificates can be added
  later as one more source for `trust:`.
- Inventing new cryptography. We use existing, reviewed protocols and
  Rust crates.

## Design

### How schemes work (short)

A daemon registers a name with the kernel, for example `tcp`. From then on
every `open`, `read` and `write` on `tcp:…` goes to that daemon. A daemon
can itself open other schemes, so `secure:` can be built on `tcp:` and
`udp:`. The process namespace decides which schemes a program sees: a
program without `keys:` in its namespace can't reach any key.

### Building blocks

| Scheme | Daemon | Job |
|---|---|---|
| `keys:` | `keyd` | Key store: create key pairs, sign, derive shared secrets. Never returns a private key. |
| `secure:` | `securenetd` | Encrypted, authenticated connections on top of `tcp:` and `udp:`. |
| `trust:` | `trustd` | Which public key belongs to which device, service or name. |
| `accounts:` | `accountd` | On the service side: which public keys may log in as which user. |
| `consent:` | `consentd` | Asks the user to approve: a registration, sharing fields, a new device. |
| `profile:` | `profiled` | Personal data fields, given out only after consent and only the requested fields. |
| `sync:` | `syncd` | One user on several machines: device keys, pairing, revoking. |

Only `keys:` and `secure:` are needed for the first step. The others are
added when a use needs them.

### `keys:`

```
keys:/<owner>/<name>/pub        read: public key (text, base64)
keys:/<owner>/<name>/sign       write data, read the signature
keys:/<owner>/<name>/dh         write a peer public key, read the shared secret
keys:/<owner>/new               write "<name> <type>" to create a key
```

- Key types: Ed25519 for signing, X25519 for key exchange, P-256 for
  Matter and iOS (Secure Enclave keys are P-256).
- Keys are stored encrypted on disk. Where the board has a secure element
  or TPM, `keyd` uses it; the Pi 3 has none, so the key file is protected
  by the file system rights and, for user keys, a passphrase.
- Randomness comes from `rand:`; `keyd` refuses to create keys until it is
  seeded.
- Rights: a program may use only the keys its namespace and user allow.

### `secure:`

```
secure:/[2001:db8::1]:7000          open: connect, do the handshake
secure:/[2001:db8::1]:7000/peer     read: the verified public key of the peer
secure:listen/7000                  open: accept connections on port 7000
```

- Protocol: the [Noise protocol framework](https://noiseprotocol.org/),
  pattern `IK` when the peer key is known and `XX` for first contact. Noise
  is small, has no certificates and no options to get wrong, and is
  proven in WireGuard. Rust (`snow`) and Swift libraries exist.
- The handshake uses a key from `keys:` through its `dh` file, so
  `securenetd` never holds the long-term private key.
- After the handshake the program reads and writes plain data.
- Over UDP the same handshake runs with a small framing, for protocols
  like CoAP or Matter-style messages.

### `trust:`

```
trust:/devices/<name>      the pinned public key of a device
trust:/services/<name>     the pinned public key of a service
trust:/roots/<name>        a root key (for example a Matter fabric root)
```

Sources: a code or QR code scanned at registration, trust on first use
(with a warning when the key changes), and later DNS records or
certificates.

### `accounts:`

```
accounts:/<user>/keys       one public key per line, with a label
accounts:/<user>/register   write a registration request (key + signed token)
accounts:/tokens/new        read: a new one-time registration token
```

A general `authorized_keys`. `sshd`, `login` and any service can use it.

### `consent:`

A request is a short text plus choices. `consentd` shows it where the user
is: the console, the screen, or a paired phone. The answer goes back to
the asking daemon.

## Flows

### 1. Register a phone app with a service on Redox

1. On Redox, the service reads a token from `accounts:/tokens/new` and
   shows a QR code: its public key (from `keys:`), its IPv6 address and the
   token.
2. The phone app scans it and pins the service key. This answers "is this
   the right server" without certificates.
3. The app makes its own key in the phone's secure storage (Secure Enclave
   on iOS, P-256).
4. The app connects over `secure:` (Noise `IK`, the service key is known)
   and sends its public key with the token signed.
5. `accountd` checks the token (one-time, short-lived) and stores the key.

### 2. Secure access afterwards

1. The app connects over `secure:` with both keys known.
2. The service reads `…/peer`, looks the key up in `accounts:` and knows
   the user.
3. The service contains no crypto code. No passwords are sent.

### 3. One user on several Redox machines

1. The user has a user key; each machine has its own device key.
2. The user key signs each device key. A new machine is paired with an
   existing one through a short code, over `secure:`.
3. `syncd` copies `trust:`, `accounts:` and `profile:` encrypted between
   the user's machines.
4. Services accept any device key signed by the user key, so a phone
   registered once works with all machines.
5. A lost device is revoked: `syncd` spreads the revocation.

## Matter

Matter uses its own security protocols, which are fixed by the standard,
so `secure:` does not replace them. The same building blocks still hold
the keys and the trust:

| Matter needs | Building block |
|---|---|
| Fabric root key (RCAC) and the controller's operational key (NOC) | `keys:` (P-256) |
| Trusted device makers (PAA roots), fabric roots | `trust:/roots/` |
| Approving the commissioning of a new device | `consent:` |
| PASE (setup code, SPAKE2+) and CASE (certificate session) over UDP | a `matter:` daemon that uses `keys:` for every signature |
| Group keys for multicast | `keys:` |

## Security considerations

- One daemon holds all keys, so it must stay small and be reviewed. It
  runs as its own user with only the schemes it needs.
- Programs can use keys but not read them; a compromised program can sign
  while it runs, but can't take the key away.
- Registration tokens are one-time and expire after minutes.
- Trust on first use warns loudly when a known key changes.
- Key loss: user keys need a backup (a printed recovery code or a second
  device); device keys can simply be revoked and replaced.
- Separate keys per service, so services can't link a user across services.

## Alternatives considered

- **TLS everywhere with certificates:** needs a certificate system and a
  correct clock (the Pi has no battery clock), and many options to get
  wrong.
- **IPsec in the network stack:** complex key management, problems with NAT.
- **Each program does its own crypto:** today's state, which this RFC
  wants to replace.

## Open questions

- Noise `IK`/`XX` only, or also TLS 1.3 with raw public keys (RFC 7250) for
  peers that already speak TLS?
- How the namespace rights for `keys:` are configured, per program or per
  user.
- How `consent:` reaches the user on a headless Pi (console, phone, web).
- Backup format for user keys.

## Plan

| Step | Release | Content |
|---|---|---|
| 1 | v0.6.0 | `keyd` with `keys:` (Ed25519, X25519), `securenetd` with `secure:` (Noise over TCP), test Redox to Redox in QEMU |
| 2 | v0.7.0 | `accountd`, registration with a QR code; `sshd` host key from `keys:`, SSH login with keys from `accounts:`; a small phone test app |
| 3 | v0.8.0 | `trustd`; Matter keys and fabric roots in `keys:` and `trust:`; `consentd` for commissioning |
| 4 | later | `syncd` (several machines), `profiled` (personal data) |
