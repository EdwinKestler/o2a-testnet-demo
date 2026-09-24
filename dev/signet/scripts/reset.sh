#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
printf 'Delete both signet data volumes? [yes/no] '
read -r answer
if [ "$answer" != "yes" ]; then
  echo "Nothing was deleted."
  exit 0
fi
docker compose --file docker-compose.yml down --volumes
echo "Stack stopped and both data volumes deleted."
