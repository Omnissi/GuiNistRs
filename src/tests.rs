use nistrs::prelude::*;

use std::{sync::Mutex, time::Duration};

#[derive(Copy, Clone)]
pub struct TestParam {
    pub _min_value: usize,
    pub _max_value: usize,
    pub value: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TestStat {
    pub ratio: f64,
    pub p_distr: [usize; 10],
}

pub const NUMBER_OF_TEST: usize = 15;

pub type ResultTestsStat = [Vec<TestStat>; NUMBER_OF_TEST];
pub type TestFn = fn(&BitsData, Option<TestParam>) -> Result<Vec<TestResultT>, Option<String>>;

#[derive(Copy, Clone)]
pub struct NistWrapper {
    pub name: &'static str,
    pub enable: bool,
    pub param: Option<TestParam>,
    pub test_cb: TestFn,
}

lazy_static! {
    pub static ref TESTS: Mutex<[NistWrapper; NUMBER_OF_TEST]> = {
        Mutex::new([
            NistWrapper {
                name: "Frequency",
                enable: true,
                param: None,
                test_cb: |data, _| Ok(vec![frequency_test(data)]),
            },
            NistWrapper {
                name: "BlockFrequency",
                enable: true,
                param: Some(TestParam {
                    _min_value: 10,
                    _max_value: 1000,
                    value: 128,
                }),
                test_cb: |data, param| match block_frequency_test(data, param.unwrap().value) {
                    Ok(v) => Ok(vec![v]),
                    Err(s) => Err(Some(s)),
                },
            },
            NistWrapper {
                name: "Runs",
                enable: true,
                param: None,
                test_cb: |data, _| Ok(vec![runs_test(data)]),
            },
            NistWrapper {
                name: "LongestRunOfOnes",
                enable: true,
                param: None,
                test_cb: |data, _| match longest_run_of_ones_test(data) {
                    Ok(v) => Ok(vec![v]),
                    Err(s) => Err(Some(s)),
                },
            },
            NistWrapper {
                name: "Rank",
                enable: true,
                param: None,
                test_cb: |data, _| match rank_test(data) {
                    Ok(v) => Ok(vec![v]),
                    Err(s) => Err(Some(s)),
                },
            },
            NistWrapper {
                name: "FFT",
                enable: true,
                param: None,
                test_cb: |data, _| Ok(vec![fft_test(data)]),
            },
            NistWrapper {
                name: "NonOverlappingTemplate",
                enable: true,
                param: Some(TestParam {
                    _min_value: 2,
                    _max_value: 16,
                    value: 9,
                }),
                test_cb: |data, param| match non_overlapping_template_test(
                    data,
                    param.unwrap().value,
                ) {
                    Ok(v) => Ok(v),
                    Err(s) => Err(Some(s)),
                },
            },
            NistWrapper {
                name: "Overlapping",
                enable: true,
                param: Some(TestParam {
                    _min_value: 2,
                    _max_value: 1000,
                    value: 9,
                }),
                test_cb: |data, param| {
                    Ok(vec![overlapping_template_test(data, param.unwrap().value)])
                },
            },
            NistWrapper {
                name: "Universal",
                enable: true,
                param: None,
                test_cb: |data, _| Ok(vec![universal_test(data)]),
            },
            NistWrapper {
                name: "LinearComplexity",
                enable: true,
                param: Some(TestParam {
                    _min_value: 500,
                    _max_value: 1000,
                    value: 500,
                }),
                test_cb: |data, param| Ok(vec![linear_complexity_test(data, param.unwrap().value)]),
            },
            NistWrapper {
                name: "Serial",
                enable: true,
                param: Some(TestParam {
                    _min_value: 2,
                    _max_value: 128,
                    value: 16,
                }),
                test_cb: |data, param| Ok(serial_test(data, param.unwrap().value).to_vec()),
            },
            NistWrapper {
                name: "ApproximateEntropy",
                enable: true,
                param: Some(TestParam {
                    _min_value: 2,
                    _max_value: 100,
                    value: 10,
                }),
                test_cb: |data, param| {
                    Ok(vec![approximate_entropy_test(data, param.unwrap().value)])
                },
            },
            NistWrapper {
                name: "CumulativeSums",
                enable: true,
                param: None,
                test_cb: |data, _| Ok(cumulative_sums_test(data).to_vec()),
            },
            NistWrapper {
                name: "RandomExcursions",
                enable: true,
                param: None,
                test_cb: |data, _| match random_excursions_test(data) {
                    Ok(v) => Ok(v.to_vec()),
                    Err(_) => Err(None),
                },
            },
            NistWrapper {
                name: "RandomExcursionsVariant",
                enable: true,
                param: None,
                test_cb: |data, _| match random_excursions_variant_test(data) {
                    Ok(v) => Ok(v.to_vec()),
                    Err(_) => Err(None),
                },
            },
        ])
    };
    pub static ref STOP_FLAG: Mutex<bool> = Mutex::new(false);
    pub static ref COMPLETE_BLOCKS: Mutex<usize> = Mutex::new(0_usize);
    pub static ref AVR_TIME_TO_BLOCK: Mutex<u128> = Mutex::new(0_u128);
    pub static ref TOTAL_TIME: Mutex<Duration> = Mutex::new(Duration::new(0, 0));
    pub static ref NUMBERS_OF_BLOCKS: Mutex<usize> = Mutex::new(0_usize);
}
