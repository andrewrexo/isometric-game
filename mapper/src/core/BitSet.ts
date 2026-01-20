// Efficient bit storage for collision data
export class BitSet {
  private data: Uint8Array;
  private size: number;

  constructor(size: number) {
    this.size = size;
    this.data = new Uint8Array(Math.ceil(size / 8));
  }

  get(index: number): boolean {
    if (index < 0 || index >= this.size) return false;
    const byteIndex = Math.floor(index / 8);
    const bitIndex = index % 8;
    return (this.data[byteIndex] & (1 << bitIndex)) !== 0;
  }

  set(index: number, value: boolean): void {
    if (index < 0 || index >= this.size) return;
    const byteIndex = Math.floor(index / 8);
    const bitIndex = index % 8;
    if (value) {
      this.data[byteIndex] |= 1 << bitIndex;
    } else {
      this.data[byteIndex] &= ~(1 << bitIndex);
    }
  }

  toggle(index: number): boolean {
    const current = this.get(index);
    this.set(index, !current);
    return !current;
  }

  clear(): void {
    this.data.fill(0);
  }

  fill(): void {
    this.data.fill(0xff);
  }

  // Get the raw Uint8Array for storage/transfer
  getRaw(): Uint8Array {
    return this.data;
  }

  // Set from raw Uint8Array
  setRaw(data: Uint8Array): void {
    const copyLength = Math.min(data.length, this.data.length);
    for (let i = 0; i < copyLength; i++) {
      this.data[i] = data[i];
    }
  }

  // Encode to Base64 string for JSON storage
  toBase64(): string {
    let binary = '';
    for (let i = 0; i < this.data.length; i++) {
      binary += String.fromCharCode(this.data[i]);
    }
    return btoa(binary);
  }

  // Decode from Base64 string
  static fromBase64(base64: string, size: number): BitSet {
    const bitset = new BitSet(size);
    try {
      const binary = atob(base64);
      const copyLength = Math.min(binary.length, bitset.data.length);
      for (let i = 0; i < copyLength; i++) {
        bitset.data[i] = binary.charCodeAt(i);
      }
    } catch {
      // Invalid base64, return empty bitset
    }
    return bitset;
  }

  // Create from boolean array
  static fromBoolArray(arr: boolean[]): BitSet {
    const bitset = new BitSet(arr.length);
    for (let i = 0; i < arr.length; i++) {
      if (arr[i]) {
        bitset.set(i, true);
      }
    }
    return bitset;
  }

  // Convert to boolean array
  toBoolArray(): boolean[] {
    const arr: boolean[] = new Array(this.size);
    for (let i = 0; i < this.size; i++) {
      arr[i] = this.get(i);
    }
    return arr;
  }

  // Count set bits
  count(): number {
    let count = 0;
    for (let i = 0; i < this.data.length; i++) {
      let byte = this.data[i];
      while (byte) {
        count += byte & 1;
        byte >>= 1;
      }
    }
    return count;
  }

  // Clone the bitset
  clone(): BitSet {
    const cloned = new BitSet(this.size);
    cloned.data.set(this.data);
    return cloned;
  }

  getSize(): number {
    return this.size;
  }
}
