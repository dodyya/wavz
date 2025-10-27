# waveZ
Group Name: Paul and David
Group members and NetIDs: pauljm2, davidf14 respectively

Project introduction:
We plan to implement the Fast Fourier Transform algorithm, and apply it to visualizing .wav audio files. 

#### Goals:
- Understand and implement recursive FFT
- Understand and implement the .wav parser
- Create a visually pleasing demo; audio visualization along with playback in the best case, and static spectrogram image in the worst case
- We have chosen to work on this project, because it yields itself well to working in parallel. This project will contain significant amounts of both theoretical and applied depth. 


#### Technical Overview:
- [ ] Parser for .wav files, figuring out what parts of the spec are relevant to our use case.
- [ ] Internal representation of audio signal
- [ ] Fast Fourier Transform implementation, potentially with a complex number struct, tested on synthetic audio (e.g. pure sine wave sums)

==== CHECKPOINT 1; 11/10-11/15 ====
- [ ] Visualization pipeline with `pixels`, visualize the signal as a waveform, pipe to image file
- [ ] Static visualization of synthetic frequency data, adapt visualization to arbitrary vector lengths
- [ ] Plumbing file information into the transform, adapting transform to work on non-powers of 2, and specific time ranges (non-global)
- [ ] Static visualization of real .wav audio

==== CHECKPOINT 2; 12/1-12/5 ====
- [ ] UI improvements; see wave and frequencies in the same window, use some external crate
- [ ] .wav playback (using external crate), attempt to sync sliding window frequencies with audio

==== SUBMISSION; 12/10 ====

This project is approved by Linus Torvalds 👍
