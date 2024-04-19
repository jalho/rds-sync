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
  Copypasta from https://github.com/golang/go/issues/34416
*/
type PrefixWriter struct {
	f func() string
	w io.Writer
}
func (p PrefixWriter) Write(b []byte) (n int, err error) {
	if n, err = p.w.Write([]byte(p.f())); err != nil {
		return
	}
	nn, err := p.w.Write(b)
	return n + nn, err
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
		f: func() string { return "[" + time.Now().Format(time.RFC3339) + "] " },
		w: log.Writer(),
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
