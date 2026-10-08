use mutsukitube_embedded_player::EmbeddedMpvPlayer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[MutsukiTube] Initializing libmpv...");

    let player = EmbeddedMpvPlayer::new_headless().map_err(std::io::Error::other)?;

    println!("[MutsukiTube] libmpv initialized.");

    player.set_volume(50.0).map_err(std::io::Error::other)?;

    println!("[MutsukiTube] Volume property set.");

    player.stop().map_err(std::io::Error::other)?;

    println!("[MutsukiTube] Smoke test completed.");

    Ok(())
}
