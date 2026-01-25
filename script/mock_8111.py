#!/usr/bin/env uv run
# /// script
# requires-python = ">=3.8"
# dependencies = [
#     "requests",
#     "psutil",
# ]
# ///

import argparse
import datetime
import http.server
import json
import os
import socketserver
import sys
from pathlib import Path
from typing import Dict, List, Optional, Union

import psutil
import requests

# Constants
DEFAULT_SOURCE_PORT = 8111
DEFAULT_MOCK_PORT = 8112
DEFAULT_DATA_FILE = Path(__file__).parent / "mock_data.json"
ENDPOINTS = [
    "/state",
    "/indicators",
    "/map_obj.json",
    "/map_info.json"
]

def get_process_info_on_port(port: int) -> Optional[str]:
    """Finds the process using the specified port and returns its info."""
    for conn in psutil.net_connections(kind='inet'):
        if conn.laddr.port == port and conn.status == 'LISTEN':
            try:
                proc = psutil.Process(conn.pid)
                return f"{proc.name()} (PID: {conn.pid})"
            except (psutil.NoSuchProcess, psutil.AccessDenied):
                return f"PID: {conn.pid}"
    return None

class MockDataRepository:
    """Manages the storage and retrieval of mocked flight data."""
    
    def __init__(self, file_path: Path):
        self.file_path = file_path

    def save(self, data: Dict[str, Union[dict, str]]):
        with open(self.file_path, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=4, ensure_ascii=False)
        print(f"Data successfully saved to {self.file_path}")

    def load(self) -> Dict[str, Union[dict, str]]:
        if not self.file_path.exists():
            raise FileNotFoundError(f"Mock data file not found: {self.file_path}")
        with open(self.file_path, "r", encoding="utf-8") as f:
            return json.load(f)

class WarThunderClient:
    """Handles communication with the live War Thunder data port."""
    
    def __init__(self, host: str = "127.0.0.1", port: int = DEFAULT_SOURCE_PORT):
        self.base_url = f"http://{host}:{port}"

    def capture_all(self, endpoints: List[str]) -> Dict[str, Union[dict, str]]:
        print(f"Capturing flight data from {self.base_url}...")
        captured_data = {}
        for ep in endpoints:
            url = f"{self.base_url}{ep}"
            try:
                response = requests.get(url, timeout=2)
                response.raise_for_status()
                try:
                    captured_data[ep] = response.json()
                except ValueError:
                    captured_data[ep] = response.text
                print(f" [+] Captured {ep}")
            except Exception as e:
                print(f" [!] Failed to capture {ep}: {e}")
        return captured_data

class MockRequestHandler(http.server.BaseHTTPRequestHandler):
    """
    Custom handler for the mock server.
    Ensures byte-perfect compatibility with VoidMei's fragile parsing.
    """
    
    repository: MockDataRepository = None
    _cached_data: Optional[Dict] = None

    def log_message(self, format, *args):
        pass # Silence logs

    @property
    def data(self):
        if self._cached_data is None:
            self._cached_data = self.repository.load()
        return self._cached_data

    def do_GET(self):
        try:
            content = self.data.get(self.path)
            if content is not None:
                # VoidMei's StringHelper expects exactly ONE space after the colon: "key": value
                # Default json.dumps provides exactly this: separators=(', ', ': ')
                # But we want to avoid spaces after commas to keep the buffer payload small.
                response_body = json.dumps(content, separators=(',', ': ')) if isinstance(content, (dict, list)) else str(content)
                body_bytes = response_body.encode('utf-8')
                
                # VoidMei's HttpHelper.java (sendGetFast) expects exactly 6 lines before data.
                # 1. Status Line
                # 2-5. Exactly 4 Header Lines (We must avoid keyword "type" or "valid" in labels)
                # 6. A blank line
                # We manually construct the entire raw response to ensure bit-perfect delivery.
                date_str = datetime.datetime.now(datetime.timezone.utc).strftime('%a, %d %b %Y %H:%M:%S GMT')
                
                raw_response = [
                    b"HTTP/1.1 200 OK",
                    f"Date: {date_str}".encode('ascii'),
                    b"Server: MockServer/2.0",
                    b"Connection: close",
                    f"Content-Length: {len(body_bytes)}".encode('ascii'),
                    b"",  # This becomes the blank line
                    body_bytes
                ]
                
                # Send the entire chunk at once to minimize chances of truncated reads in Java
                self.wfile.write(b"\r\n".join(raw_response))
            else:
                self.send_error(404)
        except Exception as e:
            # We don't want to use send_error here if we might have already written part of a response
            print(f"Server Error handling {self.path}: {e}")

class ReusableTCPServer(socketserver.TCPServer):
    """TCP Server that allows quick rebinding of the same address/port."""
    allow_reuse_address = True

def run_capture(args):
    client = WarThunderClient(port=args.source_port)
    repo = MockDataRepository(args.file)
    data = client.capture_all(ENDPOINTS)
    if data:
        repo.save(data)
    else:
        print("Error: No data captured. Is War Thunder running?")

def run_serve(args):
    repo = MockDataRepository(args.file)
    if not args.file.exists():
        print(f"Error: {args.file} not found. Please run 'capture' first.")
        return

    # Pass the repo to the handler class
    MockRequestHandler.repository = repo
    
    print(f"Starting mock server on port {args.port}...")
    try:
        with ReusableTCPServer(("", args.port), MockRequestHandler) as server:
            print(f"Mock server active at http://localhost:{args.port}")
            print("Press Ctrl+C to stop.")
            server.serve_forever()
    except KeyboardInterrupt:
        print("\nStopping mock server.")
    except OSError as e:
        if e.errno == 98:
            proc_info = get_process_info_on_port(args.port)
            occupier = f" by {proc_info}" if proc_info else ""
            print(f"Error: Port {args.port} is already in use{occupier}.")
            print(f"Try a different port using --port, or kill the existing process.")
        else:
            raise

def main():
    parser = argparse.ArgumentParser(description="War Thunder 8111 Port Mocker")
    subparsers = parser.add_subparsers(dest="command", required=True)

    # Capture command
    cap_parser = subparsers.add_parser("capture", help="Capture live data from War Thunder")
    cap_parser.add_argument("--source-port", type=int, default=DEFAULT_SOURCE_PORT, help="Game data port (default: 8111)")
    cap_parser.add_argument("--file", type=Path, default=DEFAULT_DATA_FILE, help="Path to save captured data")

    # Serve command
    serve_parser = subparsers.add_parser("serve", help="Start mock server")
    serve_parser.add_argument("--port", type=int, default=DEFAULT_MOCK_PORT, help="Mock server port (default: 8112)")
    serve_parser.add_argument("--file", type=Path, default=DEFAULT_DATA_FILE, help="Path to mock data file")

    args = parser.parse_args()

    if args.command == "capture":
        run_capture(args)
    elif args.command == "serve":
        run_serve(args)

if __name__ == "__main__":
    main()
