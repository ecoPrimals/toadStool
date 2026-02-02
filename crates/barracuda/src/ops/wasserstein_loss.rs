//! WassersteinLoss - Wasserstein distance for GANs
//!
//! Earth Mover's Distance between distributions.
//! More stable GAN training (WGAN).

pub async fn wasserstein_loss(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    critic_real: &[f32], // Critic scores for real samples
    critic_fake: &[f32], // Critic scores for fake samples
) -> Result<(f32, f32), Box<dyn std::error::Error>> {
    if critic_real.is_empty() || critic_fake.is_empty() {
        return Err("Empty input".into());
    }

    // Discriminator loss: maximize D(real) - D(fake)
    // = minimize -(D(real) - D(fake))
    let real_mean: f32 = critic_real.iter().sum::<f32>() / critic_real.len() as f32;
    let fake_mean: f32 = critic_fake.iter().sum::<f32>() / critic_fake.len() as f32;

    let disc_loss = -(real_mean - fake_mean);

    // Generator loss: maximize D(fake) = minimize -D(fake)
    let gen_loss = -fake_mean;

    Ok((disc_loss, gen_loss))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_wasserstein_loss() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let critic_real = vec![0.8; 64];
        let critic_fake = vec![-0.5; 64];
        let (disc_loss, _gen_loss) =
            wasserstein_loss(&dev.device, &dev.queue, &critic_real, &critic_fake)
                .await
                .unwrap();
        assert!(disc_loss < 0.0); // Should be negative (maximizing margin)
    }
}
