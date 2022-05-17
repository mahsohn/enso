package org.enso.base.statistics;

/**
 * Class to compute covariance and correlations between series.
 */
public class CorrelationStats {
  private long count = 0;
  private double totalX = 0.0;
  private double totalXX = 0.0;
  private double totalY = 0.0;
  private double totalYY = 0.0;
  private double totalXY = 0.0;

  private void append(Double x, Double y) {
    if (x == null || x.isNaN() || y == null || y.isNaN()) {
      return;
    }

    count++;
    totalX += x;
    totalXX += x * x;
    totalY += y;
    totalYY += y * y;
    totalXY += x * y;
  }

  public double covariance() {
    if (count < 2) {
      return Double.NaN;
    }

    return (totalXY - totalX * totalY / count) / count;
  }

  public double pearsonCorrelation() {
    if (count < 2) {
      return Double.NaN;
    }

    double n_stdev_x = Math.sqrt(count * totalXX - totalX * totalX);
    double n_stdev_y = Math.sqrt(count * totalYY - totalY * totalY);
    return (count * totalXY - totalX * totalY) / (n_stdev_x * n_stdev_y);
  }

  public double rSquared() {
    double correl = this.pearsonCorrelation();
    return  correl * correl;
  }

  /***
   * Create the CorrelationStats between two series
   * @param x Array of X values
   * @param y Array of Y values
   * @return CorrelationStats object for the 2 series.
   */
  public static CorrelationStats compute(Double[] x, Double[] y) {
    if (x.length != y.length) {
      throw new IllegalArgumentException("Left and right lengths are not the same.");
    }

    CorrelationStats output = new CorrelationStats();
    for (int i = 0; i < x.length; i++) {
      output.append(x[i], y[i]);
    }
    return output;
  }

  public static CorrelationStats[][] computeMatrix(Double[][] data) {
    int len = data[0].length;

    for (int i = 1; i < data.length; i++) {
      if (data[i].length != len) {
        throw new IllegalArgumentException("Data lengths are not consistent.");
      }
    }

    CorrelationStats[][] output = new CorrelationStats[data.length][];
    for (int i = 0; i < data.length; i++) {
      output[i] = new CorrelationStats[data.length];
      for (int j = 0; j < data.length; j++) {
        if (j < i) {
          output[i][j] = output[j][i];
        } else {
          output[i][j] = compute(data[i], data[j]);
        }
      }
    }
    return output;
  }
}
