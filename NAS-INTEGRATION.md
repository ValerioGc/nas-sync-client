# NAS integration guide

How NAS Sync works with the gateway, the launcher, Security Center and `nas-sync-manager`.

## Pairing

1. `GET /health` and `GET /auth/ca.crt` on the HTTPS port of the gateway. The
   certificate-authority fingerprint requires confirmation.
2. `POST /auth/login`, `POST /auth/confirm` and `POST /auth/clients` with
   `{ name, syncId }`, where `syncId` is the identity of the Syncthing of the computer. The
   gateway issues a device certificate and answers `{ device, certificate, key, authority }` in
   PEM. The computer is a device of Security Center, marked as a sync client.
3. `POST /client/sync/pair`, with that certificate. The gateway identifies the computer to
   `nas-sync-manager` in `x-nas-client`, and only a certificate never used before may pair.

With mTLS on, the gateway refuses step 2 because the computer has no certificate yet.

## Sync routes

`/client/sync/{pair,folders,syncs,syncs/<id>}` reach `/apps/nas-sync-manager/api/client/...` only
with an active certificate of a sync client. The gateway drops `x-nas-client` from every other
request and starts the application when it does not answer.

## Revocation

Revoking or removing the computer in Security Center drops its engine and its syncs from
`nas-sync-manager`, retried every five minutes until the application confirms. The files stay on
both sides. The client shows that the computer is no longer allowed and offers a new connection.

## Network

The NAS opens its sync ports to the local network while at least one sync is active. The client
dials the NAS on TCP 22000 and listens on the loopback only.
