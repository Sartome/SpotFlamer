use self_update::backends::github::Update;
use self_update::cargo_crate_version;
use tracing::{info, error};

pub fn spawn_update_checker() {
    std::thread::spawn(|| {
        if let Err(e) = check_and_update() {
            error!("Erreur lors de la mise à jour : {}", e);
        }
    });
}

fn check_and_update() -> Result<(), Box<dyn std::error::Error>> {
    let pub_key = include_str!("../assets/minisign.pub");
    
    // Extraire seulement la dernière ligne de la clé publique (ignorer le commentaire)
    let pub_key_clean = pub_key.lines().last().unwrap_or("").trim();
    
    // Decode the base64 public key (minisign keys are 42 bytes when decoded, the last 32 are the key)
    use base64ct::{Base64, Encoding};
    let mut decoded = [0u8; 64];
    let decoded_slice = Base64::decode(pub_key_clean, &mut decoded).map_err(|_| "Erreur base64")?;
    if decoded_slice.len() < 32 {
        return Err("Clé publique trop courte".into());
    }
    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&decoded_slice[decoded_slice.len() - 32..]);

    let status = Update::configure()
        .repo_owner("Sartome")
        .repo_name("SpotFlamer")
        .bin_name("spotflamer.exe")
        .show_download_progress(false)
        .current_version(cargo_crate_version!())
        .verifying_keys([key_bytes])
        .build()?
        .update()?;

    if status.updated() {
        info!("Mise à jour réussie vers la version {}", status.version());
        // L'application sera redémarrée à la prochaine exécution.
    } else {
        info!("L'application est à jour (v{})", status.version());
    }

    Ok(())
}
