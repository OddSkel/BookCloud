pub fn pearson_correlation(x: &[f64], y: &[f64]) -> f64 {
    if x.len() != y.len() || x.len() < 2 { return 0.0; }
    let n = x.len() as f64;
    let mx = x.iter().sum::<f64>() / n;
    let my = y.iter().sum::<f64>() / n;
    let cov: f64  = x.iter().zip(y).map(|(a, b)| (a - mx) * (b - my)).sum::<f64>() / n;
    let sx  = (x.iter().map(|a| (a - mx).powi(2)).sum::<f64>() / n).sqrt();
    let sy  = (y.iter().map(|b| (b - my).powi(2)).sum::<f64>() / n).sqrt();
    if sx == 0.0 || sy == 0.0 { return 0.0; }
    cov / (sx * sy)
}

pub fn interpret_correlation(r: f64) -> String {
    match r {
        r if r >=  0.7 => "Strong positive correlation",
        r if r >=  0.4 => "Moderate positive correlation",
        r if r >=  0.1 => "Weak positive correlation",
        r if r >  -0.1 => "No significant correlation",
        r if r >  -0.4 => "Weak negative correlation",
        r if r >  -0.7 => "Moderate negative correlation",
        _              => "Strong negative correlation",
    }.to_string()
}