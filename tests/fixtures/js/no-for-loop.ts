export function doubleAll(nums: number[]): number[] {
  const result: number[] = [];
  for (let i = 0; i < nums.length; i++) {
    result.push(nums[i] * 2);
  }
  return result;
}
