package main

import (
  "log"
  "net"
  "os"
  "time"
  "io"
)

/*
  Logger shenanigans to make it emit ISO formatted timestamps.
*/
type PrefixWriter struct {
	write_prefix func() string
	writer io.Writer
}
func (self PrefixWriter) Write(payload []byte) (n int, err error) {
  prefix := []byte(self.write_prefix());
  bytes_written_prefix, err_write_prefix := self.writer.Write(prefix)
	if err_write_prefix != nil {
		return
	}

	bytes_written_payload, err_write_payload := self.writer.Write(payload)
	return bytes_written_prefix + bytes_written_payload, err_write_payload
}

/*
  WHAT DO?

  This program receives messages about game events over a Unix domain socket.
  The sender is another process on the same host (namely a Carbon plugin loaded
  into RustDedicated).
*/
func main() {
  // set up logger
	log.SetFlags(0)
  log_writer := PrefixWriter{
		write_prefix: func() string { return "[" + time.Now().Format(time.RFC3339) + "] " },
		writer: log.Writer(),
	}
	log.SetOutput(log_writer)

  // set up stats receiving socket
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

  // get messages and do stuff about them...
  buffer := make([]byte, 1024)
  for {
    n, _, err := conn.ReadFromUnix(buffer)
    if err != nil {
      log.Fatal("Error reading from Unix socket:", err)
    }
    log.Printf("Received: '%s'\n", buffer[:n])
  }
}
