//! CalcHelper 的 Rust 移植 (src/prog/util/CalcHelper.java)
//! POC 期间已随派生量翻译, 归口 vm-core (A 类)

/// Java CalcHelper.SimpleMovingAverage 渐进均值: 预热段全量平均, 之后环形覆盖增量更新
pub struct SimpleMovingAverage {
    data: Vec<f64>,
    cnt: usize,
    avg: f64,
}

impl SimpleMovingAverage {
    pub fn new(num: usize) -> Self {
        SimpleMovingAverage {
            data: vec![0.0; num],
            cnt: 0,
            avg: 0.0,
        }
    }

    pub fn add_new_data(&mut self, ndata: f64) -> f64 {
        let n = self.data.len();
        if self.cnt < n {
            // 添加数据
            self.data[self.cnt] = ndata;
            self.cnt += 1;
            self.avg = self.data[..self.cnt].iter().sum::<f64>() / self.cnt as f64;
        } else {
            let ridx = self.cnt % n;
            self.avg += (ndata - self.data[ridx]) / n as f64;
            self.data[ridx] = ndata;
            self.cnt += 1;
        }
        self.avg
    }
}

#[cfg(test)]
mod tests;
