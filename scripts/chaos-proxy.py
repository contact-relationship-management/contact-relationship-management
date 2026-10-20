#!/usr/bin/env python3
"""A TCP proxy in front of PostgreSQL that scripts/chaos.sh steers from the shell.

    chaos-proxy.py LISTEN_PORT CONTROL_DIR

It reads small files in CONTROL_DIR on every chunk, so changes apply to connections
that are already open:

    target    the port to forward to (default: the first line of the file, required)
    mode      forward | blackhole | reset | delay:MILLISECONDS
                blackhole  accept and hold connections, forward nothing (a partition
                           that drops packets: no error, no answer)
                reset      close every connection and refuse new ones (a partition that
                           sends resets)
                delay:N    wait N ms before passing each chunk either way (a slow link)
    killpid   if present, a process id. On the next COMMIT a client sends, the proxy
              forwards it, keeps PostgreSQL's reply from reaching the client, kills that
              process with SIGKILL and removes the file. The transaction has committed
              and the service never learns it. One shot.

Standard library only. Not for production; it exists to break things on purpose.
"""
import asyncio
import os
import signal
import sys

PORT = int(sys.argv[1])
CONTROL = sys.argv[2]


def read(name, default=""):
    try:
        with open(os.path.join(CONTROL, name)) as handle:
            return handle.read().strip() or default
    except FileNotFoundError:
        return default


def mode():
    return read("mode", "forward")


async def pause_for_mode():
    """Honour delay and blackhole before passing a chunk; False means drop the connection."""
    while True:
        current = mode()
        if current == "reset":
            return False
        if current == "blackhole":
            await asyncio.sleep(0.1)
            continue
        if current.startswith("delay:"):
            await asyncio.sleep(int(current.split(":", 1)[1]) / 1000)
        return True


async def pipe(reader, writer, state, client_to_server):
    try:
        while True:
            chunk = await reader.read(65536)
            if not chunk:
                break
            if not await pause_for_mode():
                break
            if client_to_server and b"COMMIT\x00" in chunk and read("killpid"):
                state["hold"] = True
                writer.write(chunk)
                await writer.drain()
                await asyncio.sleep(0.3)  # PostgreSQL has the COMMIT and runs it
                pid = int(read("killpid"))
                os.remove(os.path.join(CONTROL, "killpid"))
                os.kill(pid, signal.SIGKILL)
                break
            if not client_to_server and state["hold"]:
                continue
            writer.write(chunk)
            await writer.drain()
    except (ConnectionError, asyncio.CancelledError):
        pass
    finally:
        writer.close()


async def handle(client_reader, client_writer):
    if mode() == "reset":
        client_writer.close()
        return
    while mode() == "blackhole":
        await asyncio.sleep(0.1)
        if client_writer.is_closing():
            return
    try:
        server_reader, server_writer = await asyncio.open_connection("127.0.0.1", int(read("target")))
    except OSError:
        client_writer.close()
        return
    state = {"hold": False}
    await asyncio.gather(
        pipe(client_reader, server_writer, state, True),
        pipe(server_reader, client_writer, state, False),
    )


async def main():
    server = await asyncio.start_server(handle, "127.0.0.1", PORT)
    async with server:
        await server.serve_forever()


asyncio.run(main())
