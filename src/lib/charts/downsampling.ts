/**
 * High-performance downsampling algorithms for chart rendering
 *
 * Implements LTTB (Largest Triangle Three Buckets) algorithm
 * for visually-accurate downsampling of time series data.
 */

/**
 * LTTB (Largest Triangle Three Buckets) downsampling algorithm
 *
 * Reduces data points while preserving visual characteristics.
 * Based on: https://skemman.is/bitstream/1946/15343/3/SS_MSthesis.pdf
 *
 * @param data - Array of [x, y] points
 * @param threshold - Target number of points (must be >= 2)
 * @returns Downsampled array of points
 */
export function lttbDownsample(
	data: ArrayLike<number>,
	dataLength: number,
	getX: (i: number) => number,
	getY: (i: number) => number,
	threshold: number
): [number, number][] {
	if (threshold >= dataLength || threshold < 2) {
		// Return all points if threshold is larger than data or too small
		const result: [number, number][] = [];
		for (let i = 0; i < dataLength; i++) {
			const y = getY(i);
			if (!Number.isNaN(y)) {
				result.push([getX(i), y]);
			}
		}
		return result;
	}

	const sampled: [number, number][] = [];

	// Bucket size (leaving room for first and last points)
	const bucketSize = (dataLength - 2) / (threshold - 2);

	// Always include first point
	let aIndex = 0;
	sampled.push([getX(0), getY(0)]);

	for (let i = 0; i < threshold - 2; i++) {
		// Calculate bucket boundaries
		const bucketStart = Math.floor((i + 1) * bucketSize) + 1;
		const bucketEnd = Math.min(Math.floor((i + 2) * bucketSize) + 1, dataLength - 1);

		// Calculate average point for next bucket (used as reference)
		const nextBucketStart = Math.floor((i + 2) * bucketSize) + 1;
		const nextBucketEnd = Math.min(Math.floor((i + 3) * bucketSize) + 1, dataLength - 1);

		let avgX = 0;
		let avgY = 0;
		let avgCount = 0;

		for (let j = nextBucketStart; j < nextBucketEnd && j < dataLength; j++) {
			const y = getY(j);
			if (!Number.isNaN(y)) {
				avgX += getX(j);
				avgY += y;
				avgCount++;
			}
		}

		if (avgCount > 0) {
			avgX /= avgCount;
			avgY /= avgCount;
		}

		// Find point in current bucket that creates largest triangle
		let maxArea = -1;
		let maxAreaIndex = bucketStart;

		const aX = getX(aIndex);
		const aY = getY(aIndex);

		for (let j = bucketStart; j < bucketEnd && j < dataLength; j++) {
			const pointY = getY(j);
			if (Number.isNaN(pointY)) continue;

			const pointX = getX(j);

			// Calculate triangle area using cross product
			const area = Math.abs(
				(aX - avgX) * (pointY - aY) - (aX - pointX) * (avgY - aY)
			);

			if (area > maxArea) {
				maxArea = area;
				maxAreaIndex = j;
			}
		}

		// Add the point with the largest triangle area
		sampled.push([getX(maxAreaIndex), getY(maxAreaIndex)]);
		aIndex = maxAreaIndex;
	}

	// Always include last point
	sampled.push([getX(dataLength - 1), getY(dataLength - 1)]);

	return sampled;
}

/**
 * Fast min-max downsampling for overview/zoomed-out views
 *
 * For each bucket, keeps both min and max values to preserve peaks.
 * Faster than LTTB but slightly less visually accurate.
 *
 * @param data - Source data array
 * @param dataLength - Number of data points
 * @param getX - Function to get X value at index
 * @param getY - Function to get Y value at index
 * @param threshold - Target number of points
 * @returns Downsampled array of points
 */
export function minMaxDownsample(
	dataLength: number,
	getX: (i: number) => number,
	getY: (i: number) => number,
	threshold: number
): [number, number][] {
	if (threshold >= dataLength || threshold < 2) {
		const result: [number, number][] = [];
		for (let i = 0; i < dataLength; i++) {
			const y = getY(i);
			if (!Number.isNaN(y)) {
				result.push([getX(i), y]);
			}
		}
		return result;
	}

	const sampled: [number, number][] = [];
	const bucketSize = dataLength / (threshold / 2); // Each bucket contributes min+max

	for (let i = 0; i < threshold / 2; i++) {
		const bucketStart = Math.floor(i * bucketSize);
		const bucketEnd = Math.min(Math.floor((i + 1) * bucketSize), dataLength);

		let minY = Infinity;
		let maxY = -Infinity;
		let minIndex = bucketStart;
		let maxIndex = bucketStart;

		for (let j = bucketStart; j < bucketEnd; j++) {
			const y = getY(j);
			if (Number.isNaN(y)) continue;

			if (y < minY) {
				minY = y;
				minIndex = j;
			}
			if (y > maxY) {
				maxY = y;
				maxIndex = j;
			}
		}

		// Add in order (min first if it comes first in data)
		if (minIndex <= maxIndex) {
			if (minY !== Infinity) sampled.push([getX(minIndex), minY]);
			if (maxY !== -Infinity && maxIndex !== minIndex) sampled.push([getX(maxIndex), maxY]);
		} else {
			if (maxY !== -Infinity) sampled.push([getX(maxIndex), maxY]);
			if (minY !== Infinity && minIndex !== maxIndex) sampled.push([getX(minIndex), minY]);
		}
	}

	return sampled;
}

/**
 * Calculate optimal sample count based on container width
 *
 * @param containerWidth - Width of chart container in pixels
 * @param pixelsPerPoint - Target pixels between points (default: 2)
 * @returns Optimal number of sample points
 */
export function calculateSampleCount(containerWidth: number, pixelsPerPoint = 2): number {
	// Minimum 100 points, maximum based on container width
	return Math.max(100, Math.min(5000, Math.floor(containerWidth / pixelsPerPoint)));
}
