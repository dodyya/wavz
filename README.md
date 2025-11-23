# waveZ

Group Name: Paul and David

Group members and NetIDs: pauljm2, davidf14 respectively

Project introduction:
We plan to implement the Fast Fourier Transform algorithm, and apply it to visualizing .wav audio files.

#### Goals:

- Understand and implement recursive FFT
- Understand and implement Cooley-Tukey "iterative" FFT
- Understand and implement the .wav parser
- Create a visually pleasing demo; audio visualization along with playback ~~in the best case, and static spectrogram image in the worst case~~
- We have chosen to work on this project, because it yields itself well to working in parallel. This project will contain significant amounts of both theoretical and applied depth.

==== COMPLETE BY CHECKPOINT 1; 11/10-11/15 ====

- [x] Parser for .wav files, figuring out what parts of the spec are relevant to our use case.
- [x] Internal representation of audio signal
- [x] Fast Fourier Transform implementation, potentially with a complex number struct, tested on synthetic audio (e.g. pure sine wave sums)

==== COMPLETE BY CHECKPOINT 2; 12/1-12/5 ====

- [x] Visualization pipeline with `pixels`, visualize the signal as a waveform, pipe to image file
- [x] Static visualization of synthetic frequency data, ~~adapt visualization to arbitrary vector lengths~~
- [x] Plumbing file information into the transform,
- [x] specific time ranges (non-global)
- [x] Static visualization of real .wav audio
- [x] Time-synced scrolling
- [x] Play/pause/scroll along visualization
- [ ] Playback indicator (scrolling vertical line)
- [ ] Graceful pause upon song end

==== COMPLETE BY SUBMISSION; 12/10 ====

- [ ] Variable y-ranges in visualization, log scale?
- [ ] Sync audio and visualization
- [ ] Alternative mode that displays "bouncing" waveform
      ~~ - [ ] UI improvements; see wave and frequencies in the same window, use some external crate~~
- [ ] .wav playback (using external crate), attempt to sync sliding window frequencies with audio

==§§ POTENTIAL EXTENSIONS §§==

- [ ] Reasonable UX that allows you to pick the file
- [ ] Inverse FFT
- [ ] Modulate real audio data
- [ ] Inverse parser that writes to file
- [ ] Box-select frequencies in viewer, move/scale/resize, and export resulting .wav
