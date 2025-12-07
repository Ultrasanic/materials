package main

import (
	"crypto/rsa"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"

	"github.com/golang-jwt/jwt/v5"
)

type UsernamePassword struct {
	Username string `json:"username"`
	Password string `json:"password"`
}

type AuthHandlers struct {
	passwords  map[string][16]byte
	privateKey *rsa.PrivateKey
	publicKey  *rsa.PublicKey
}

func NewAuthHandlers(privateKeyFile string, publicKeyFile string) *AuthHandlers {
	private, err := os.ReadFile(privateKeyFile)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	public, err := os.ReadFile(publicKeyFile)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	privateKey, err := jwt.ParseRSAPrivateKeyFromPEM(private)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	publicKey, err := jwt.ParseRSAPublicKeyFromPEM(public)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	return &AuthHandlers{
		passwords:  make(map[string][16]byte),
		privateKey: privateKey,
		publicKey:  publicKey,
	}
}

func (h *AuthHandlers) signup(w http.ResponseWriter, req *http.Request) {
	if req.Method != http.MethodPost {
		w.WriteHeader(http.StatusBadRequest)
		fmt.Fprintf(w, "signup can be done only with POST HTTP method")
		return
	}
	body := make([]byte, req.ContentLength)
	read, err := req.Body.Read(body)
	defer req.Body.Close()
	if read != int(req.ContentLength) {
		w.WriteHeader(http.StatusBadRequest)
		return
	}
	if err != io.EOF {
		w.WriteHeader(http.StatusInternalServerError)
		fmt.Fprintf(w, "Error reading body: %v", err)
		return
	}
	creds := UsernamePassword{}
	err = json.Unmarshal(body, &creds)
	if err != nil {
		w.WriteHeader(http.StatusBadRequest)
		fmt.Fprintf(w, "Error unmarshalling body: %v", err)
		return
	}

	// TODO: check if user exists, create user and generate token

	w.Write([]byte("TODO")) // jwt token string
}

func (h *AuthHandlers) login(w http.ResponseWriter, req *http.Request) {
	if req.Method != http.MethodPost {
		w.WriteHeader(http.StatusBadRequest)
		fmt.Fprintf(w, "login can be done only with POST HTTP method")
		return
	}
	body := make([]byte, req.ContentLength)
	read, err := req.Body.Read(body)
	defer req.Body.Close()
	if read != int(req.ContentLength) {
		w.WriteHeader(http.StatusBadRequest)
		return
	}
	if err != io.EOF {
		w.WriteHeader(http.StatusInternalServerError)
		fmt.Fprintf(w, "Error reading body: %v", err)
		return
	}
	creds := UsernamePassword{}
	err = json.Unmarshal(body, &creds)
	if err != nil {
		w.WriteHeader(http.StatusBadRequest)
		fmt.Fprintf(w, "Error unmarshalling body: %v", err)
		return
	}

	// TODO: check if user exists, check password and generate token

	w.Write([]byte("TODO")) // jwt token string
}

func (h *AuthHandlers) whoami(w http.ResponseWriter, req *http.Request) {
	username := "TODO" // TODO: get username from jwt token payload

	// TODO: check if user exists

	w.Write([]byte("Hello, " + username))
}

func main() {
	privateKeyFile := flag.String("private", "", "path to private key file")
	publicKeyFile := flag.String("public", "", "path to public key file")
	port := flag.Int("port", 8091, "HTTP server port")
	flag.Parse()

	if port == nil {
		fmt.Fprintln(os.Stderr, "Port is required")
		os.Exit(1)
	}

	if privateKeyFile == nil || *privateKeyFile == "" {
		fmt.Fprintln(os.Stderr, "Please provide a path to private key file")
		os.Exit(1)
	}

	if publicKeyFile == nil || *publicKeyFile == "" {
		fmt.Fprintln(os.Stderr, "Please provide a path to public key file")
		os.Exit(1)
	}

	absPrivateKeyFile, err := filepath.Abs(*privateKeyFile)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}

	absPublicKeyFile, err := filepath.Abs(*publicKeyFile)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}

	authHandlers := NewAuthHandlers(absPrivateKeyFile, absPublicKeyFile)

	http.HandleFunc("/signup", authHandlers.signup)
	http.HandleFunc("/login", authHandlers.login)
	http.HandleFunc("/whoami", authHandlers.whoami)

	fmt.Println("Starting server on port", *port, "with private key file", absPrivateKeyFile, "and public key file", absPublicKeyFile)

	if err = http.ListenAndServe(fmt.Sprintf(":%d", *port), nil); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
