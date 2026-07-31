import lamejs from 'lamejs';

/**
 * Converts a 24kHz 16-bit Mono/Stereo WAV ArrayBuffer into a high-quality MP3 byte array.
 * @param wavBuffer Raw ArrayBuffer of the source WAV file
 * @param bitrateKbps MP3 bitrate target (default: 192 kbps)
 * @returns Uint8Array containing encoded MP3 bytes
 */
export function convertWavBufferToMp3(wavBuffer: ArrayBuffer, bitrateKbps: number = 192): Uint8Array {
  const view = new DataView(wavBuffer);
  
  // Verify RIFF header
  const riff = String.fromCharCode(view.getUint8(0), view.getUint8(1), view.getUint8(2), view.getUint8(3));
  if (riff !== 'RIFF') {
    throw new Error('Invalid WAV file header (missing RIFF signature)');
  }

  const channels = view.getUint16(22, true);
  const sampleRate = view.getUint32(24, true);

  // Find data subchunk
  let dataOffset = 12;
  while (dataOffset < view.byteLength - 8) {
    const subChunkId = String.fromCharCode(
      view.getUint8(dataOffset),
      view.getUint8(dataOffset + 1),
      view.getUint8(dataOffset + 2),
      view.getUint8(dataOffset + 3)
    );
    const subChunkSize = view.getUint32(dataOffset + 4, true);
    if (subChunkId === 'data') {
      dataOffset += 8;
      break;
    }
    dataOffset += 8 + subChunkSize;
  }

  if (dataOffset >= view.byteLength) {
    dataOffset = 44; // Fallback standard header size
  }

  const pcmSamples = new Int16Array(wavBuffer, dataOffset, Math.floor((wavBuffer.byteLength - dataOffset) / 2));
  const mp3encoder = new (lamejs as any).Mp3Encoder(channels, sampleRate, bitrateKbps);
  const mp3Chunks: Uint8Array[] = [];

  const sampleBlockSize = 1152 * 5;
  for (let i = 0; i < pcmSamples.length; i += sampleBlockSize) {
    const chunk = pcmSamples.subarray(i, i + sampleBlockSize);
    const mp3buf = mp3encoder.encodeBuffer(chunk);
    if (mp3buf.length > 0) {
      mp3Chunks.push(new Uint8Array(mp3buf));
    }
  }

  const endBuf = mp3encoder.flush();
  if (endBuf.length > 0) {
    mp3Chunks.push(new Uint8Array(endBuf));
  }

  const totalLength = mp3Chunks.reduce((acc, chunk) => acc + chunk.length, 0);
  const result = new Uint8Array(totalLength);
  let currentOffset = 0;
  for (const chunk of mp3Chunks) {
    result.set(chunk, currentOffset);
    currentOffset += chunk.length;
  }

  return result;
}
