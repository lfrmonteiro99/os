# AuroraOS Prototype

Minimal Rust prototype for a boot-to-shell flow:

- `init`: boot entrypoint binary
- `shell`: interactive terminal UI frontend
- `ipc`: transport frame types for shell/control-plane communication
- `control-plane`: command parser and dispatcher
- `svc-manager`: service registry, lifecycle, metrics, and snapshots

## Run

```bash
cargo run -p init
```

## Native Desktop Preview (macOS-style mock)

One command:

```bat
scripts\run-desktop.cmd
```

Abre uma janela desktop nativa (Rust + egui).
Features atuais: janelas movíveis/redimensionáveis, snap com teclas `L`/`R`, botões macOS (vermelho=fechar, amarelo=minimizar para o dock, verde=maximizar/restaurar), e restore pelo dock.

Para ver telemetria real no desktop (estado de serviÃ§os):

1. Terminal A:
```bash
cargo run -p init -- --daemon --listen 127.0.0.1:7878 --auth-token topsecret
```
2. Terminal B:
```bat
set AURORA_TOKEN=topsecret && scripts\run-desktop.cmd
```

Run with a startup profile:

```bash
cargo run -p init -- --profile profiles/default.services
```

Run with autosave on shell exit:

```bash
cargo run -p init -- --autosave profiles/last.services
```

Run a startup command script:

```bash
cargo run -p init -- --script profiles/bootstrap.commands
```

Run script only (CI mode, no prompt):

```bash
cargo run -p init -- --script profiles/bootstrap.commands --no-interactive
```

Run as daemon:

```bash
cargo run -p init -- --daemon --listen 127.0.0.1:7878
```

Run as daemon with auth token required:

```bash
cargo run -p init -- --daemon --listen 127.0.0.1:7878 --auth-token topsecret
```

Run daemon with audit log and command cap:

```bash
cargo run -p init -- --daemon --listen 127.0.0.1:7878 --auth-token topsecret --audit-log logs/audit.log --max-commands 50
```

Use JSON audit format with rotation:

```bash
cargo run -p init -- --daemon --listen 127.0.0.1:7878 --auth-token topsecret --audit-log logs/audit.log --audit-format json --audit-max-bytes 1048576
```

Add per-IP window rate limit:

```bash
cargo run -p init -- --daemon --listen 127.0.0.1:7878 --auth-token topsecret --ip-rate-limit 100 --ip-rate-window-sec 60
```

Add idle timeout for inactive clients:

```bash
cargo run -p init -- --daemon --listen 127.0.0.1:7878 --auth-token topsecret --idle-timeout-sec 30
```

Connect shell to daemon:

```bash
cargo run -p init -- --connect 127.0.0.1:7878
```

Connect with auth token:

```bash
cargo run -p init -- --connect 127.0.0.1:7878 --auth-token topsecret
```

Connect and run remote script only:

```bash
cargo run -p init -- --connect 127.0.0.1:7878 --script profiles/bootstrap.commands --no-interactive
```

Show CLI options:

```bash
cargo run -p init -- --help
```

## Shell commands

- `help`
- `status`
- `list`
- `register <name>`
- `start <name>`
- `stop <name>`
- `boot`
- `uptime`
- `health`
- `events [limit]`
- `history [limit]`
- `clear-events`
- `shutdown`
- `save <path>`
- `load <path>`
- `exit`

## Test

```bash
cargo test
```
