# Aleo Stratum Mining Protocol

Version: 3.0.0 (Mainnet)

This document describes the stratum protocol for Aleo pool mining.

Currently, the only publicly defined pool mining protocol for Aleo network is the implementation of the official
operator / prover nodes. However, it's a part of the binary peer-to-peer protocol, and as it's getwork-based and
transmits the full block template, the protocol overhead is large. The provers also use up peer slots of the operator,
which might affect the network syncing on the operator side.

Some private mining protocols has emerged during the incentivized Testnet2 period, however they are all independently
developed and there was no interoperability between them.

Stratum protocol is a widely used protocol for cryptocurrency mining, but many cryptocurrencies has suffered from not
having a standardized stratum protocol - even though the stratum protocol is simple enough, there is still room for
different variants of the protocol to appear.

This document will try to define a standard stratum protocol for Aleo pool mining.

## Specification

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED",  "MAY", and "
OPTIONAL" in this document are to be interpreted as described
in [RFC 2119](https://datatracker.ietf.org/doc/html/rfc2119).

The stratum protocol uses the [JSON-RPC 2.0](https://www.jsonrpc.org/specification) specification.

All communication between the miner and the pool is over a TCP connection. All messages send by either end are JSON
messages, delimited by character `\n`. This means each message MUST be encoded in one line; `\n` characters MUST NOT
appear in the message itself.

JSON-RPC 2.0 specification has defined the format of request and response messages. Consult the specification for
details of the required members of the messages.

Following several existing stratum protocol specifications, this document defines the following application-specific
error codes:

- 20 - Other/Unknown
- 21 - Job not found (=stale)
- 22 - Duplicate share
- 23 - Low difficulty share
- 24 - Unauthorized worker
- 25 - Not subscribed

### Methods

### `mining.subscribe`

This method is used by miners to initiate sessions with the mining pool.

Request:

```json
{
    "id": 1,
    "method": "mining.subscribe",
    "params": [
        "MINER_USER_AGENT",
        "PROTOCOL_VERSION",
        "SESSION_ID"
    ]
}
```

`MINER_USER_AGENT` (string): The name and version of the miner software.

`PROTOCOL_VERSION` (string): The version of the stratum protocol. MUST be in the format of `"AleoStratum/3.0.0"`. This
field is used for the miner to request a specified version of the protocol and for the server to decide if the requested
protocol version is supported.

`SESSION_ID` (string): The previous session ID the miner wants to resume. SHOULD be `null` if the miner wants to
initiate a new session.

Response:

```json
{
    "id": 1,
    "result": [
        "SESSION_ID",
        "SERVER_NONCE",
        "ADDRESS"
    ],
    "error": null
}
```

`SESSION_ID` (string): If the server supports session resuming, this field MUST be the same as the one sent by the miner
in the request if there is one. Otherwise, it MUST be a new unique session ID. The server MUST set this field to `null`
if it doesn't support session resuming.

`SERVER_COUNTER` (hex) `(Testnet3)`: Server counter set by the server. See [Counters](#Counters) for more information.
MUST be `null` if there is no server nonce set.

`ADDRESS` (string): The address of the pool. See [Address](#Address) for more information.

### `mining.authorize`

This method is used by miners to authorize themselves to the mining pool. The miner MUST authorize at least one worker
before submitting shares.

Request:

```json
{
    "id": 1,
    "method": "mining.authorize",
    "params": [
        "WORKER_NAME",
        "WORKER_PASSWORD"
    ]
}
```

`WORKER_NAME` (string): The name of the worker.

`WORKER_PASSWORD` (string): The password of the worker.

Response:

```json
{
    "id": 1,
    "result": RESULT,
    "error": null
}
```

`RESULT` (bool): If the worker is authorized, this field MUST be `true`. Otherwise, it MUST be `null`, and the server
SHOULD give reasons in the `error` object.

### `mining.set_target`

This notification is used by the server to set the target share difficulty for the jobs after the current one.

Request:

```json
{
    "id": null,
    "method": "mining.set_target",
    "params": [
        TARGET
    ]
}
```

`TARGET` (int): The target share difficulty. This is the `proof_target` for the coinbase puzzle, which MUST be
between `1` and `2 ^ 64 - 1`, where `1` means easiest.

### `mining.notify`

This notification is used by the server to notify the miner about the new job.

Request:

```json
{
    "id": null,
    "method": "mining.notify",
    "params": [
        "JOB_ID",
        "EPOCH_HASH",
        "ADDRESS",
        "CLEAN_JOBS"
    ]
}
```

`JOB_ID` (string): The job ID.

`EPOCH_HASH` (hex): The epoch hash.

`ADDRESS` (string): The address of the pool. MAY be null if the address is not changed. See [Address](#Address) for more
information.

`CLEAN_JOBS` (bool): If this field is `true`, the miner MUST discard all previous jobs as they are already stale.
Otherwise, the miner MAY keep the previous jobs.

See [Notify](#Notify) for more information.

### `mining.submit`

This method is used by miners to submit shares to the pool.

Request:

```json
{
    "id": 1,
    "method": "mining.submit",
    "params": [
        "WORKER_NAME",
        "JOB_ID",
        "COUNTER"
    ]
}
```

`WORKER_NAME` (string): The name of the authorized worker.

`JOB_ID` (string): The job ID.

`COUNTER` (string): The counter of the solution.

See [Submit](#Submit) for information about potential attacks.

## Comments

### Counters

In Mainnet, the counter is a `u64` type. Pool operators might want to set a server nonce prefix to prevent miners from
mining on the same nonce. The miner MUST use the server nonce prefix to construct the proof if it is set. The server
nonce MUST be set to `null` if there is no server nonce set.

### Notify

The input of the prove function in Mainnet is `epoch_hash`, `address` and `counter`. Those three parameters are used to
construct the solution. Therefore, the prover need to have the address of the pool.

### Address

The pool server sends the pool address in the response of `subscribe` message as the new proving mechanism requires it.

Some pools might want to change the pool address between different blocks, thus every `notify` message might contain an
address to override the previous ones.

### Submit

Due to how the puzzle works, the pool server MUST calculate the solution target instead of accepting the target from the
prover. Calculating the target requires solving the puzzle again with the submitted counter, so the pool server SHOULD
implement some defense mechanism against DoS attacks from provers (by submitting random counters).

## Version History

### 3.0.0

Updated to reflect the changes in Mainnet.

### 2.0.0

Updated to reflect the changes in Testnet3.

### 1.0.0

Initial version for Testnet2.