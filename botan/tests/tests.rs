extern crate botan;

#[test]
fn test_version() {
    let version = botan::Version::new().unwrap();

    /*
    If we are running against a released version we know it must be at
    least 2.8 since we require APIs added after the 2.7 release.
    */

    assert_eq!(version.major, 2);
    assert!(version.minor >= 8);
    assert!(version.release_date == 0 || version.release_date >= 20181001);
    assert!(version.ffi_api >= 20180713);

    assert!(botan::Version::supports_version(version.ffi_api));
    assert!(botan::Version::supports_version(20180713));
    assert!(!botan::Version::supports_version(20180712));

        //println!("{:?}", version);
}

#[test]
fn test_hash() {
    let hash = botan::HashFunction::new("SHA-384").unwrap();

    assert_eq!(hash.output_length().unwrap(), 48);
    assert_eq!(hash.block_size().unwrap(), 128);
    assert_eq!(hash.algo_name().unwrap(), "SHA-384");

    assert!(hash.update(&[97,98]).is_ok());

    let hash_dup = hash.duplicate().unwrap();

    assert!(hash.update(&[99]).is_ok());
    assert!(hash_dup.update(&[100]).is_ok());

    hash.clear().unwrap();

    hash.update(&[97,98,99]).unwrap();

    let digest = hash.finish().unwrap();

    assert_eq!(digest[0], 0xCB);
    assert_eq!(digest[1], 0x00);
    assert_eq!(digest[47], 0xA7);

    let digest_dup = hash_dup.finish().unwrap();

    assert_eq!(digest_dup[0], 0x5D);
    assert_eq!(digest_dup[1], 0x15);
    assert_eq!(digest_dup[47], 0xF7);

    let bad_hash = botan::HashFunction::new("BunnyHash9000");

    assert_eq!(bad_hash.is_err(), true);
    assert_eq!(*bad_hash.as_ref().unwrap_err(), botan::Error::NotImplemented);
}


#[test]
fn test_mac() {
    let mac = botan::MsgAuthCode::new("HMAC(SHA-384)").unwrap();

    let key_spec = mac.key_spec().unwrap();
    assert_eq!(mac.output_length().unwrap(), 48);
    assert_eq!(mac.algo_name().unwrap(), "HMAC(SHA-384)");

    assert!(key_spec.is_valid_keylength(20));

    mac.set_key(&vec![0xAA; 20]).unwrap();

    mac.update(&vec![0xDD; 1]).unwrap();
    mac.update(&vec![0xDD; 29]).unwrap();
    mac.update(&vec![0xDD; 20]).unwrap();

    let r = mac.finish().unwrap();

    println!("{:?}", r);

    assert_eq!(r[0], 0x88);
    assert_eq!(r[1], 0x06);
    assert_eq!(r[47], 0x27);

}

#[test]
fn test_block_cipher() {
    let bc = botan::BlockCipher::new("AES-128").unwrap();

    assert_eq!(bc.algo_name().unwrap(), "AES-128");
    assert_eq!(bc.block_size().unwrap(), 16);

    let key_spec = bc.key_spec().unwrap();

    assert!(key_spec.is_valid_keylength(20) == false);
    assert!(key_spec.is_valid_keylength(16));

    assert_eq!(bc.set_key(&vec![0; 32]).unwrap_err(), botan::Error::InvalidKeyLength);

    bc.set_key(&vec![0; 16]).unwrap();

    let input = vec![0; 16];

    let ctext = bc.encrypt_blocks(&input).unwrap();

    let expected = vec![0x66, 0xe9, 0x4b, 0xd4, 0xef, 0x8a, 0x2c, 0x3b, 0x88, 0x4c, 0xfa, 0x59, 0xca, 0x34, 0x2b, 0x2e];
    assert_eq!(ctext, expected);

    let ptext = bc.decrypt_blocks(&ctext).unwrap();

    assert_eq!(ptext, input);
}

#[test]
fn test_cipher() {
    let cipher = botan::Cipher::new("AES-128/GCM", botan::CipherDirection::Encrypt).unwrap();

    assert_eq!(cipher.tag_length(), 16);

    let zero16 = vec![0; 16];
    let zero12 = vec![0; 12];

    assert!(cipher.set_associated_data(&[1,2,3]).is_err()); // trying to set AD before key is set
    assert_eq!(cipher.set_key(&vec![0; 42]).unwrap_err(), botan::Error::InvalidKeyLength);

    cipher.set_key(&zero16).unwrap();

    cipher.set_associated_data(&[1,2,3]).unwrap();
    cipher.set_associated_data(&[]).unwrap();


    let ctext = cipher.process(&zero12, &zero16).unwrap();

    assert_eq!(ctext, botan::hex_decode("0388DACE60B6A392F328C2B971B2FE78AB6E47D42CEC13BDF53A67B21257BDDF").unwrap());

    let cipher = botan::Cipher::new("AES-128/GCM", botan::CipherDirection::Decrypt).unwrap();
    cipher.set_key(&zero16).unwrap();

    let ptext = cipher.process(&zero12, &ctext).unwrap();

    assert_eq!(ptext, zero16);
}

#[test]
fn test_chacha() {
    let cipher = botan::Cipher::new("ChaCha20", botan::CipherDirection::Encrypt).unwrap();

    assert_eq!(cipher.tag_length(), 0);

    let key_spec = cipher.key_spec().unwrap();

    assert!(key_spec.is_valid_keylength(0) == false);
    assert!(key_spec.is_valid_keylength(16));
    assert!(key_spec.is_valid_keylength(32));
    assert!(key_spec.is_valid_keylength(48) == false);

    let key = vec![0; 32];

    let expected = botan::hex_decode("76B8E0ADA0F13D90405D6AE55386BD28BDD219B8A08DED1AA836EFCC8B770DC7DA41597C5157488D7724E03FB8D84A376A43B8F41518A11CC387B669").unwrap();

    cipher.set_key(&key).unwrap();

    assert!(cipher.set_associated_data(&[1,2,3]).is_err()); // not an AEAD
    assert!(cipher.set_associated_data(&[]).is_err());

    let iv = vec![];
    let input = vec![0; expected.len()];

    let ctext = cipher.process(&iv, &input).unwrap();

    assert_eq!(ctext, expected);
}


#[test]
fn test_kdf() {

    let salt = botan::hex_decode("000102030405060708090A0B0C").unwrap();
    let label = botan::hex_decode("F0F1F2F3F4F5F6F7F8F9").unwrap();
    let secret = botan::hex_decode("0B0B0B0B0B0B0B0B0B0B0B0B0B0B0B0B0B0B0B0B0B0B").unwrap();
    let expected_output = botan::hex_decode("3CB25F25FAACD57A90434F64D0362F2A2D2D0A90CF1A5A4C5DB02D56ECC4C5BF34007208D5B887185865").unwrap();

    let output = botan::kdf("HKDF(SHA-256)", expected_output.len(), &secret, &salt, &label).unwrap();

    assert_eq!(output, expected_output);
}

#[test]
fn test_pbkdf() {

    let salt = botan::hex_decode("0001020304050607").unwrap();
    let iterations = 10000;
    let passphrase = "xyz";
    let expected_output = botan::hex_decode("DEFD2987FA26A4672F4D16D98398432AD95E896BF619F6A6B8D4ED").unwrap();

    let output = botan::pbkdf("PBKDF2(SHA-256)", expected_output.len(), passphrase, &salt, iterations).unwrap();

    assert_eq!(output, expected_output);
}

#[test]
fn test_scrypt() {

    let salt = botan::hex_decode("4E61436C").unwrap();
    let n = 1024;
    let r = 8;
    let p = 16;
    let passphrase = "password";
    let expected_output = botan::hex_decode("fdbabe1c9d3472007856e7190d01e9fe7c6ad7cbc8237830e77376634b3731622e").unwrap();

    let output = botan::scrypt(expected_output.len(), passphrase, &salt, n, r, p).unwrap();

    assert_eq!(output, expected_output);
}

#[test]
fn test_hex() {
    let raw = vec![1,2,3,255,42,23];
    assert_eq!(botan::hex_encode(&raw).unwrap(), "010203FF2A17");
    assert_eq!(botan::hex_decode("010203FF2A17").unwrap(), raw);
}

#[test]
fn test_rng() {
    let rng = botan::RandomNumberGenerator::new_system().unwrap();

    let read1 = rng.read(10).unwrap();
    let read2 = rng.read(10).unwrap();

    assert!(read1 != read2);
}

#[test]
fn test_certs() {
    let cert_bits = botan::hex_decode("3082035A30820305A003020102020101300C06082A8648CE3D04030105003050310B3009060355040613024445310D300B060355040A0C0462756E64310C300A060355040B0C03627369310D300B06035504051304343536373115301306035504030C0C637363612D6765726D616E79301E170D3037303731393135323731385A170D3238303131393135313830305A3050310B3009060355040613024445310D300B060355040A0C0462756E64310C300A060355040B0C03627369310D300B06035504051304343536373115301306035504030C0C637363612D6765726D616E79308201133081D406072A8648CE3D02013081C8020101302806072A8648CE3D0101021D00D7C134AA264366862A18302575D1D787B09F075797DA89F57EC8C0FF303C041C68A5E62CA9CE6C1C299803A6C1530B514E182AD8B0042A59CAD29F43041C2580F63CCFE44138870713B1A92369E33E2135D266DBB372386C400B0439040D9029AD2C7E5CF4340823B2A87DC68C9E4CE3174C1E6EFDEE12C07D58AA56F772C0726F24C6B89E4ECDAC24354B9E99CAA3F6D3761402CD021D00D7C134AA264366862A18302575D0FB98D116BC4B6DDEBCA3A5A7939F020101033A000401364A4B0F0102E9502AB9DC6855D90B065A6F5E5E48395F8309D57C11ABAFF21756607EF6757EC9886CA222D83CA04B1A99FA43C5A9BCE1A38201103082010C30360603551D11042F302D8118637363612D6765726D616E79406273692E62756E642E646586116661783A2B343932323839353832373232300E0603551D0F0101FF040403020106301D0603551D0E041604140096452DE588F966C4CCDF161DD1F3F5341B71E7301F0603551D230418301680140096452DE588F966C4CCDF161DD1F3F5341B71E730410603551D20043A30383036060904007F0007030101013029302706082B06010505070201161B687474703A2F2F7777772E6273692E62756E642E64652F6373636130120603551D130101FF040830060101FF020100302B0603551D1004243022800F32303037303731393135323731385A810F32303237313131393135313830305A300C06082A8648CE3D0403010500034100303E021D00C6B41E830217FD4C93B59E9E2B13734E09C182FA63FAEE4115A8EDD5021D00D27938DA01B8951A9064A1B696AEDF181B74968829C138F0EB2F623B").unwrap();

    let cert = botan::Certificate::load(&cert_bits).unwrap();

    let key_id = botan::hex_decode("0096452DE588F966C4CCDF161DD1F3F5341B71E7").unwrap();
    assert_eq!(cert.serial_number().unwrap(), vec![1]);
    assert_eq!(cert.authority_key_id().unwrap(), key_id);
    assert_eq!(cert.subject_key_id().unwrap(), key_id);

    let pubkey = cert.public_key().unwrap();

    assert_eq!(pubkey.algo_name().unwrap(), "ECDSA");
}

#[test]
fn test_bcrypt() {
    let pass = "password";
    let rng = botan::RandomNumberGenerator::new_system().unwrap();

    let bcrypt1 = botan::bcrypt_hash(pass, &rng, 10).unwrap();

    assert_eq!(bcrypt1.len(), 60);

    let bcrypt2 = botan::bcrypt_hash(pass, &rng, 10).unwrap();

    assert_eq!(bcrypt2.len(), 60);

    assert!(bcrypt1 != bcrypt2);

    assert!(botan::bcrypt_verify(pass, &bcrypt1).unwrap());
    assert!(botan::bcrypt_verify(pass, &bcrypt2).unwrap());

    assert_eq!(botan::bcrypt_verify("passwurd", &bcrypt2).unwrap(), false);
}

#[test]
fn test_pubkey() {
    let rng = botan::RandomNumberGenerator::new_system().unwrap();

    let ecdsa_key = botan::Privkey::create("ECDSA", "secp256r1", &rng).unwrap();

    assert!(ecdsa_key.check_key(&rng).unwrap(), true);
    assert_eq!(ecdsa_key.algo_name().unwrap(), "ECDSA");

    assert!(ecdsa_key.get_field("n").is_err());
    assert_eq!(ecdsa_key.get_field("order"),
               botan::MPI::new_from_str("0xFFFFFFFF00000000FFFFFFFFFFFFFFFFBCE6FAADA7179E84F3B9CAC2FC632551"));

    let pub_key = ecdsa_key.pubkey().unwrap();

    assert_eq!(pub_key.algo_name().unwrap(), "ECDSA");

    let bits = ecdsa_key.der_encode().unwrap();
    let pem = ecdsa_key.pem_encode().unwrap();
    assert!(pem.starts_with("-----BEGIN PRIVATE KEY-----\n"));
    assert!(pem.ends_with("-----END PRIVATE KEY-----\n"));

    let pub_bits = pub_key.der_encode().unwrap();
    let pub_pem = pub_key.pem_encode().unwrap();
    assert!(pub_pem.starts_with("-----BEGIN PUBLIC KEY-----\n"));
    assert!(pub_pem.ends_with("-----END PUBLIC KEY-----\n"));

    let loaded_key = botan::Privkey::load_der(&bits).unwrap();
    assert!(loaded_key.check_key(&rng).unwrap(), true);

    let loaded_bits = loaded_key.der_encode().unwrap();
    let loaded_pub_key = loaded_key.pubkey().unwrap();
    assert_eq!(loaded_pub_key.algo_name().unwrap(), "ECDSA");
    let loaded_pub_bits = loaded_pub_key.der_encode().unwrap();

    assert_eq!(bits, loaded_bits);
    assert_eq!(pub_bits, loaded_pub_bits);
}

#[test]
fn test_rsa() {
    let rng = botan::RandomNumberGenerator::new_system().unwrap();

    let padding = "EMSA-PKCS1-v1_5(SHA-256)";
    let msg = rng.read(32).unwrap();

    let privkey = botan::Privkey::create("RSA", "1024", &rng).unwrap();
    let pubkey = privkey.pubkey().unwrap();

    assert_eq!(privkey.get_field("e"), botan::MPI::new_from_str("65537"));
    assert_eq!(privkey.get_field("n").unwrap().bit_count().unwrap(), 1024);

    assert_eq!(pubkey.get_field("n"), privkey.get_field("n"));

    let p = privkey.get_field("p").unwrap();
    let q = privkey.get_field("q").unwrap();

    assert_eq!(p.mul(&q), privkey.get_field("n"));

    let signer = botan::Signer::new(&privkey, padding).unwrap();
    signer.update(&msg).unwrap();
    let signature = signer.finish(&rng).unwrap();

    let verifier = botan::Verifier::new(&pubkey, padding).unwrap();
    verifier.update(&msg).unwrap();
    assert_eq!(verifier.finish(&signature).unwrap(), true);

    let pubkey = botan::Pubkey::load_rsa(&privkey.get_field("n").unwrap(), &privkey.get_field("e").unwrap()).unwrap();
    let verifier = botan::Verifier::new(&pubkey, padding).unwrap();
    verifier.update(&msg).unwrap();
    assert_eq!(verifier.finish(&signature).unwrap(), true);


}

#[test]
fn test_pubkey_encryption() {

    let padding = "EMSA-PKCS1-v1_5(SHA-256)";
    let msg = [1,2,3];

    let rng = botan::RandomNumberGenerator::new_system().unwrap();
    let key = botan::Privkey::create("RSA", "1024", &rng).unwrap();

    let der = key.der_encode_encrypted("passphrase", &rng).unwrap();
    let pem = key.pem_encode_encrypted("pemword", &rng).unwrap();

    assert!(pem.starts_with("-----BEGIN ENCRYPTED PRIVATE KEY-----\n"));
    assert!(pem.ends_with("-----END ENCRYPTED PRIVATE KEY-----\n"));

    let signer = botan::Signer::new(&key, padding).unwrap();

    signer.update(&msg).unwrap();
    let sig1 = signer.finish(&rng).unwrap();

    //assert!(botan::Privkey::load_encrypted_der(&der, "i forget").is_err());

    let load = botan::Privkey::load_encrypted_der(&der, "passphrase").unwrap();
    let signer = botan::Signer::new(&load, padding).unwrap();
    signer.update(&msg).unwrap();
    let sig2 = signer.finish(&rng).unwrap();

    assert_eq!(sig1, sig2);

    let load = botan::Privkey::load_encrypted_pem(&pem, "pemword").unwrap();
    let signer = botan::Signer::new(&load, padding).unwrap();
    signer.update(&msg).unwrap();
    let sig3 = signer.finish(&rng).unwrap();

    assert_eq!(sig1, sig3);
}


#[test]
fn test_pubkey_sign() {
    let msg = vec![1,23,42];

    let rng = botan::RandomNumberGenerator::new_system().unwrap();

    let ecdsa_key = botan::Privkey::create("ECDSA", "secp256r1", &rng).unwrap();
    assert!(ecdsa_key.key_agreement_key().is_err());

    let signer = botan::Signer::new(&ecdsa_key, "EMSA1(SHA-256)").unwrap();

    signer.update(&msg).unwrap();
    let signature = signer.finish(&rng).unwrap();

    let pub_key = ecdsa_key.pubkey().unwrap();

    let verifier = botan::Verifier::new(&pub_key, "EMSA1(SHA-256)").unwrap();

    verifier.update(&[1]).unwrap();
    verifier.update(&[23, 42]).unwrap();

    assert_eq!(verifier.finish(&signature).unwrap(), true);

    verifier.update(&[1]).unwrap();
    assert_eq!(verifier.finish(&signature).unwrap(), false);

    verifier.update(&[1]).unwrap();
    verifier.update(&[23, 42]).unwrap();

    assert_eq!(verifier.finish(&signature).unwrap(), true);

}

#[test]
fn test_pubkey_encrypt() {
    let msg = vec![1,23,42];

    let rng = botan::RandomNumberGenerator::new_system().unwrap();

    let priv_key = botan::Privkey::create("RSA", "2048", &rng).unwrap();
    assert!(priv_key.key_agreement_key().is_err());
    let pub_key = priv_key.pubkey().unwrap();

    let encryptor = botan::Encryptor::new(&pub_key, "OAEP(SHA-256)").unwrap();

    let ctext = encryptor.encrypt(&msg, &rng).unwrap();
    assert_eq!(ctext.len(), 2048/8);

    let decryptor = botan::Decryptor::new(&priv_key, "OAEP(SHA-256)").unwrap();

    let ptext = decryptor.decrypt(&ctext).unwrap();

    assert_eq!(ptext, msg);
}

#[test]
fn test_pubkey_key_agreement() {

    let rng = botan::RandomNumberGenerator::new_system().unwrap();

    let a_priv = botan::Privkey::create("ECDH", "secp384r1", &rng).unwrap();
    let b_priv = botan::Privkey::create("ECDH", "secp384r1", &rng).unwrap();

    let a_pub = a_priv.key_agreement_key().unwrap();
    let b_pub = b_priv.key_agreement_key().unwrap();

    let a_ka = botan::KeyAgreement::new(&a_priv, "KDF2(SHA-384)").unwrap();
    let b_ka = botan::KeyAgreement::new(&b_priv, "KDF2(SHA-384)").unwrap();

    let salt = rng.read(16).unwrap();

    let a_key = a_ka.agree(32, &b_pub, &salt).unwrap();
    let b_key = b_ka.agree(32, &a_pub, &salt).unwrap();
    assert_eq!(a_key, b_key);

    let a_ka = botan::KeyAgreement::new(&a_priv, "Raw").unwrap();
    let b_ka = botan::KeyAgreement::new(&b_priv, "Raw").unwrap();

    let a_key = a_ka.agree(0, &b_pub, &salt).unwrap();
    let b_key = b_ka.agree(0, &a_pub, &vec![]).unwrap();

    assert_eq!(a_key, b_key);
    assert_eq!(a_key.len(), 384/8);
}

#[test]
fn test_aes_key_wrap() {
    let kek = botan::hex_decode("000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F").unwrap();
    let key = botan::hex_decode("00112233445566778899AABBCCDDEEFF000102030405060708090A0B0C0D0E0F").unwrap();

    let wrapped = botan::nist_aes_key_wrap(&kek, &key).unwrap();

    assert_eq!(botan::hex_encode(&wrapped).unwrap(),
               "28C9F404C4B810F4CBCCB35CFB87F8263F5786E2D80ED326CBC7F0E71A99F43BFB988B9B7A02DD21");

    let unwrapped = botan::nist_aes_key_unwrap(&kek, &wrapped).unwrap();

    assert_eq!(unwrapped, key);
}

#[test]
fn test_pkcs_hash_id() {
    assert!(botan::pkcs_hash_id("SHA-192").is_err());

    let id = botan::pkcs_hash_id("SHA-384").unwrap();

    assert_eq!(botan::hex_encode(&id).unwrap(),
               "3041300D060960864801650304020205000430");
}

#[test]
fn test_ct_compare() {
    let a = vec![1,2,3];

    assert_eq!(botan::const_time_compare(&a, &[1,2,3]), true);
    assert_eq!(botan::const_time_compare(&a, &[1,2,3,4]), false);
    assert_eq!(botan::const_time_compare(&a, &[1,2,4]), false);
    assert_eq!(botan::const_time_compare(&a, &a), true);
    assert_eq!(botan::const_time_compare(&a, &vec![1,2,3]), true);
}

#[test]
fn test_scrub_mem() {
    let mut v = vec![1,2,3];
    botan::scrub_mem(&mut v);
    assert_eq!(v, vec![0,0,0]);

    let mut a = [1u32, 2u32, 3u32, 2049903u32];
    botan::scrub_mem(&mut a);
    assert_eq!(a, [0,0,0,0]);
}

#[test]
fn test_mp() {
    let mut a = botan::MPI::new().unwrap();
    let mut b = botan::MPI::new().unwrap();

    assert_eq!(a.to_u32().unwrap(), 0);
    assert_eq!(b.to_u32().unwrap(), 0);

    a.set_i32(9).unwrap();
    b.set_i32(81).unwrap();

    assert_eq!(a.to_u32().unwrap(), 9);
    assert_eq!(b.to_u32().unwrap(), 81);

    let mut c = a.add(&b).unwrap();
    assert_eq!(c.to_u32().unwrap(), 90);

    let d = botan::MPI::new_from_str("0x5A").unwrap();
    assert_eq!(c, d);

    c.mul_assign(&botan::MPI::new_from_str("1030").unwrap()).unwrap();

    assert_eq!(c.to_string().unwrap(), "92700");

    assert_eq!(format!("{}", c), "92700");
    assert_eq!(format!("{:x}", c), "016a1c");
    assert_eq!(format!("{:X}", c), "016A1C");
    assert_eq!(format!("{:#x}", c), "0x016a1c");
    assert_eq!(format!("{:#X}", c), "0x016A1C");

    assert_eq!(c.to_bin().unwrap(), vec![0x01, 0x6a, 0x1c]);
}

#[test]
fn test_fpe() {
    let modulus = botan::MPI::new_from_str("1000000000").unwrap();
    let input = botan::MPI::new_from_str("939210311").unwrap();

    let key = vec![0; 32];
    let tweak = vec![0; 8];

    let fpe = botan::FPE::new_fe1(&modulus, &key, 8, false).unwrap();

    let ctext = fpe.encrypt(&input, &tweak).unwrap();

    assert_ne!(ctext, input);

    let ptext = fpe.decrypt(&ctext, &tweak).unwrap();

    assert_eq!(ptext, input);
}
