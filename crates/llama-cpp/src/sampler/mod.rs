//     /// Modifies the data array by applying a sampler to it
//     pub fn apply_sampler(&mut self, sampler: &Sampler) {
//         unsafe {
//             self.modify_as_c_llama_token_data_array(|c_llama_token_data_array| {
//                 llama_cpp_sys::llama_sampler_apply(sampler.sampler, c_llama_token_data_array);
//             });
//         }
//     }
//
//     #[must_use]
//     pub fn with_sampler(mut self, sampler: &mut Sampler) -> Self {
//         self.apply_sampler(sampler);
//         self
//     }
//
//     pub fn sample_token(&mut self, seed: u32) -> Token {
//         self.apply_sampler(&Sampler::dist(seed));
//         self.selected_token()
//             .expect("Dist sampler failed to select a token!")
//     }
//
//     pub fn sample_token_greedy(&mut self) -> Token {
//         self.apply_sampler(&Sampler::greedy());
//         self.selected_token()
//             .expect("Greedy sampler failed to select a token!")
//     }
