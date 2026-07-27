// qemu-tpm-provision creates one PCR23-bound persistent P-256 signing key.
//
// The tool talks directly to an swtpm data socket through go-tpm. It emits
// public boot-profile values and never receives or exports private key bytes.
package main

import (
	"crypto/sha256"
	"crypto/x509"
	"encoding/binary"
	"encoding/hex"
	"encoding/pem"
	"flag"
	"fmt"
	"os"

	"github.com/google/go-tpm/legacy/tpm2"
	"github.com/google/go-tpm/tpmutil"
)

const (
	policyPCRCommandCode         = uint32(0x0000017f)
	policyCommandCodeCommandCode = uint32(0x0000016c)
	signCommandCode              = uint32(0x0000015d)
)

func main() {
	socket := flag.String("socket", "", "swtpm data socket")
	handle := flag.Uint("handle", 0x81010001, "persistent TPM handle")
	publicKeyOutput := flag.String("public-key-output", "", "PEM output for the public key")
	flag.Parse()
	if *socket == "" || *publicKeyOutput == "" ||
		*handle < 0x81000000 || *handle > 0x81ffffff {
		fail("valid --socket, --public-key-output, and persistent --handle are required")
	}

	rw, err := tpmutil.OpenTPM(*socket)
	check("open swtpm socket", err)
	check("TPM2_Startup", tpm2.Startup(rw, tpm2.StartupClear))

	pcrDigest := sha256.Sum256(make([]byte, 32))
	selection := []byte{0, 0, 0x80}
	policyDigest := authorizationPolicy(selection, pcrDigest)
	template := tpm2.Public{
		Type:       tpm2.AlgECC,
		NameAlg:    tpm2.AlgSHA256,
		Attributes: tpm2.FlagFixedTPM | tpm2.FlagFixedParent | tpm2.FlagSensitiveDataOrigin | tpm2.FlagAdminWithPolicy | tpm2.FlagSign | tpm2.FlagNoDA,
		AuthPolicy: policyDigest[:],
		ECCParameters: &tpm2.ECCParams{
			Sign: &tpm2.SigScheme{
				Alg:  tpm2.AlgECDSA,
				Hash: tpm2.AlgSHA256,
			},
			CurveID: tpm2.CurveNISTP256,
		},
	}

	transient, _, _, _, _, _, err := tpm2.CreatePrimaryEx(
		rw,
		tpm2.HandleOwner,
		tpm2.PCRSelection{},
		"",
		"",
		template,
	)
	check("TPM2_CreatePrimary", err)
	persistent := tpmutil.Handle(*handle)
	check(
		"TPM2_EvictControl",
		tpm2.EvictControl(rw, "", tpm2.HandleOwner, transient, persistent),
	)

	public, name, _, err := tpm2.ReadPublic(rw, persistent)
	check("TPM2_ReadPublic", err)
	if len(name) != 34 || len(public.ECCParameters.Point.XRaw) != 32 ||
		len(public.ECCParameters.Point.YRaw) != 32 {
		fail("TPM returned an unexpected P-256 public area")
	}
	compressed := make([]byte, 33)
	compressed[0] = 0x02 | (public.ECCParameters.Point.YRaw[31] & 1)
	copy(compressed[1:], public.ECCParameters.Point.XRaw)
	key, err := public.Key()
	check("decode TPM public key", err)
	der, err := x509.MarshalPKIXPublicKey(key)
	check("encode TPM public key", err)
	check(
		"write TPM public key",
		os.WriteFile(
			*publicKeyOutput,
			pem.EncodeToMemory(&pem.Block{Type: "PUBLIC KEY", Bytes: der}),
			0o644,
		),
	)

	check("TPM2_FlushContext", tpm2.FlushContext(rw, transient))
	check("TPM2_Shutdown", tpm2.Shutdown(rw, tpm2.StartupClear))
	fmt.Printf("tpm_handle=0x%08x\n", *handle)
	fmt.Printf("tpm_name_hex=%s\n", hex.EncodeToString(name))
	fmt.Printf("state_public_key_sec1_hex=%s\n", hex.EncodeToString(compressed))
	fmt.Printf("pcr_selection_hex=%s\n", hex.EncodeToString(selection))
	fmt.Printf("pcr_digest_hex=%s\n", hex.EncodeToString(pcrDigest[:]))
	fmt.Printf("policy_digest_hex=%s\n", hex.EncodeToString(policyDigest[:]))
}

func authorizationPolicy(selection []byte, expected [32]byte) [32]byte {
	marshalledSelection := make([]byte, 10)
	binary.BigEndian.PutUint32(marshalledSelection[0:4], 1)
	binary.BigEndian.PutUint16(marshalledSelection[4:6], uint16(tpm2.AlgSHA256))
	marshalledSelection[6] = byte(len(selection))
	copy(marshalledSelection[7:], selection)

	pcr := sha256.New()
	pcr.Write(make([]byte, 32))
	writeU32(pcr, policyPCRCommandCode)
	pcr.Write(marshalledSelection)
	pcr.Write(expected[:])

	command := sha256.New()
	command.Write(pcr.Sum(nil))
	writeU32(command, policyCommandCodeCommandCode)
	writeU32(command, signCommandCode)
	var result [32]byte
	copy(result[:], command.Sum(nil))
	return result
}

type byteWriter interface {
	Write([]byte) (int, error)
}

func writeU32(writer byteWriter, value uint32) {
	var encoded [4]byte
	binary.BigEndian.PutUint32(encoded[:], value)
	_, _ = writer.Write(encoded[:])
}

func check(operation string, err error) {
	if err != nil {
		fail(fmt.Sprintf("%s: %v", operation, err))
	}
}

func fail(message string) {
	fmt.Fprintf(os.Stderr, "qemu TPM provisioning failed: %s\n", message)
	os.Exit(1)
}
