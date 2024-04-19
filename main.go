package main

import (
  "log"
  "net"
  "os"
)

func main() {
  socket_fs_path := "/tmp/rds-stats-collector.sock"
  _ = os.Remove(socket_fs_path)
  socket_unix_addr, err := net.ResolveUnixAddr("unixgram", socket_fs_path)
  if err != nil {
    log.Fatal("Error resolving Unix address:", err)
  }

  conn, err := net.ListenUnixgram("unixgram", socket_unix_addr)
  if err != nil {
    log.Fatal("Error listening on Unix socket:", err)
  }
  defer conn.Close()

  buffer := make([]byte, 1024)
  for {
    n, _, err := conn.ReadFromUnix(buffer)
    if err != nil {
      log.Fatal("Error reading from Unix socket:", err)
    }
    log.Printf("Received: '%s'\n", buffer[:n])
  }
}

