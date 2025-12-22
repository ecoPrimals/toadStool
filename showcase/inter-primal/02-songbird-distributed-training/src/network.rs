/// Simple neural network for MNIST - reuse from ml-inference

use ndarray::{Array1, Array2};

#[derive(Debug, Clone)]
pub struct SimpleNetwork {
    pub w1: Array2<f32>,
    pub b1: Array1<f32>,
    pub w2: Array2<f32>,
    pub b2: Array1<f32>,
}

impl SimpleNetwork {
    pub fn new(input_size: usize, hidden_size: usize, output_size: usize) -> Self {
        // Xavier initialization
        let scale1 = (2.0 / input_size as f32).sqrt();
        let scale2 = (2.0 / hidden_size as f32).sqrt();

        let w1 = Array2::from_shape_fn((input_size, hidden_size), |_| {
            (rand::random::<f32>() - 0.5) * scale1
        });
        let b1 = Array1::zeros(hidden_size);
        let w2 = Array2::from_shape_fn((hidden_size, output_size), |_| {
            (rand::random::<f32>() - 0.5) * scale2
        });
        let b2 = Array1::zeros(output_size);

        Self { w1, b1, w2, b2 }
    }

    pub fn forward(&self, input: ndarray::ArrayView1<f32>) -> Array1<f32> {
        // Layer 1: Linear + ReLU
        let z1 = input.dot(&self.w1) + &self.b1;
        let a1 = z1.mapv(|x| x.max(0.0)); // ReLU

        // Layer 2: Linear + Softmax
        let z2 = a1.dot(&self.w2) + &self.b2;
        let exp_z2 = z2.mapv(|x| x.exp());
        let sum_exp = exp_z2.sum();
        exp_z2 / sum_exp // Softmax
    }

    pub fn predict(&self, input: ndarray::ArrayView1<f32>) -> usize {
        let output = self.forward(input);
        output
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(idx, _)| idx)
            .unwrap()
    }
}

impl Default for SimpleNetwork {
    fn default() -> Self {
        Self::new(784, 128, 10)
    }
}

