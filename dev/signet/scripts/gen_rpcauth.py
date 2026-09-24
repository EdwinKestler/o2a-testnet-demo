#!/usr/bin/env python3
"""Write signet-infra/.env with a bitcoind rpcauth line and matching password."""

import hashlib
import hmac
import secrets
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ENV_PATH = ROOT / ".env"
USER = "signetrpc"


def main() -> None:
    password = secrets.token_hex(32)
    salt = secrets.token_hex(16)
    digest = hmac.new(salt.encode(), password.encode(), hashlib.sha256).hexdigest()
    auth_line = f"{USER}:{salt}${digest}"
    ENV_PATH.write_text(
        "\n".join(
            [
                f"RPC_USER={USER}",
                f"RPC_PASSWORD={password}",
                f"RPC_AUTH_LINE={auth_line}",
                "",
            ]
        ),
        encoding="utf-8",
    )
    ENV_PATH.chmod(0o600)
    print(f"wrote {ENV_PATH}")


if __name__ == "__main__":
    main()
