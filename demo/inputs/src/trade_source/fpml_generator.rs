//! FpML trade document generator.
//!
//! Generates FpML-formatted trade documents for testing
//! the adapter_fpml parser.

use super::{InstrumentType, TradeParams, TradeRecord};

/// FpML document generator
pub struct FpmlGenerator;

impl FpmlGenerator {
    /// Convert a trade record to FpML XML format
    pub fn to_fpml(trade: &TradeRecord) -> String {
        match &trade.params {
            TradeParams::InterestRateSwap { fixed_rate, float_index, pay_fixed } => {
                Self::generate_irs_fpml(trade, *fixed_rate, float_index, *pay_fixed)
            }
            TradeParams::CreditDefaultSwap { reference_entity, spread_bps, is_protection_buyer } => {
                Self::generate_cds_fpml(trade, reference_entity, *spread_bps, *is_protection_buyer)
            }
            TradeParams::FxForward { buy_currency, sell_currency, rate } => {
                Self::generate_fx_forward_fpml(trade, buy_currency, sell_currency, *rate)
            }
            TradeParams::EquityOption { underlying, strike, is_call } => {
                Self::generate_equity_option_fpml(trade, underlying, *strike, *is_call)
            }
            TradeParams::FxOption { currency_pair, strike, is_call } => {
                Self::generate_fx_option_fpml(trade, currency_pair, *strike, *is_call)
            }
            TradeParams::Forward { underlying, forward_price } => {
                Self::generate_equity_forward_fpml(trade, underlying, *forward_price)
            }
        }
    }

    /// Generate FX Option FpML
    fn generate_fx_option_fpml(
        trade: &TradeRecord,
        currency_pair: &str,
        strike: f64,
        is_call: bool,
    ) -> String {
        // Parse currency pair (e.g., "USDJPY" -> "USD", "JPY")
        let (ccy1, ccy2) = if currency_pair.len() >= 6 {
            (&currency_pair[0..3], &currency_pair[3..6])
        } else {
            (trade.currency.as_str(), "USD")
        };

        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<FpML xmlns="http://www.fpml.org/FpML-5/confirmation" version="5-12">
  <trade>
    <tradeHeader>
      <partyTradeIdentifier>
        <tradeId>{}</tradeId>
      </partyTradeIdentifier>
      <tradeDate>{}</tradeDate>
    </tradeHeader>
    <fxOption>
      <buyerPartyReference href="{}"/>
      <sellerPartyReference href="SELF"/>
      <optionType>{}</optionType>
      <putCurrencyAmount>
        <currency>{}</currency>
        <amount>{:.2}</amount>
      </putCurrencyAmount>
      <callCurrencyAmount>
        <currency>{}</currency>
        <amount>{:.2}</amount>
      </callCurrencyAmount>
      <strike>
        <rate>{:.6}</rate>
        <strikeQuoteBasis>CallCurrencyPerPutCurrency</strikeQuoteBasis>
      </strike>
      <europeanExercise>
        <expirationDate>{}</expirationDate>
        <expirationTime>10:00:00</expirationTime>
      </europeanExercise>
      <premium>
        <payerPartyReference href="{}"/>
        <receiverPartyReference href="SELF"/>
        <paymentDate>{}</paymentDate>
        <paymentAmount>
          <currency>{}</currency>
          <amount>0</amount>
        </paymentAmount>
      </premium>
    </fxOption>
  </trade>
</FpML>"#,
            trade.trade_id,
            trade.trade_date,
            trade.counterparty_id,
            if is_call { "Call" } else { "Put" },
            ccy2,
            trade.notional * strike,
            ccy1,
            trade.notional,
            strike,
            trade.maturity_date,
            trade.counterparty_id,
            trade.trade_date,
            trade.currency
        )
    }

    /// Generate Equity Forward FpML
    fn generate_equity_forward_fpml(
        trade: &TradeRecord,
        underlying: &str,
        forward_price: f64,
    ) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<FpML xmlns="http://www.fpml.org/FpML-5/confirmation" version="5-12">
  <trade>
    <tradeHeader>
      <partyTradeIdentifier>
        <tradeId>{}</tradeId>
      </partyTradeIdentifier>
      <tradeDate>{}</tradeDate>
    </tradeHeader>
    <equityForward>
      <buyerPartyReference href="{}"/>
      <sellerPartyReference href="SELF"/>
      <underlyer>
        <singleUnderlyer>
          <equity>
            <instrumentId instrumentIdScheme="http://www.fpml.org/spec/2002/instrument-id-RIC">{}</instrumentId>
          </equity>
          <openUnits>{:.0}</openUnits>
        </singleUnderlyer>
      </underlyer>
      <forwardPrice>
        <currency>{}</currency>
        <amount>{:.4}</amount>
      </forwardPrice>
      <valuationDate>{}</valuationDate>
    </equityForward>
  </trade>
</FpML>"#,
            trade.trade_id,
            trade.trade_date,
            trade.counterparty_id,
            underlying,
            trade.notional / forward_price,
            trade.currency,
            forward_price,
            trade.maturity_date
        )
    }

    /// Generate IRS FpML
    fn generate_irs_fpml(
        trade: &TradeRecord,
        fixed_rate: f64,
        float_index: &str,
        pay_fixed: bool,
    ) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<FpML xmlns="http://www.fpml.org/FpML-5/confirmation" version="5-12">
  <trade>
    <tradeHeader>
      <partyTradeIdentifier>
        <tradeId>{}</tradeId>
      </partyTradeIdentifier>
      <tradeDate>{}</tradeDate>
    </tradeHeader>
    <swap>
      <swapStream id="fixedLeg">
        <payerPartyReference href="{}"/>
        <calculationPeriodDates>
          <effectiveDate>{}</effectiveDate>
          <terminationDate>{}</terminationDate>
          <calculationPeriodFrequency>6M</calculationPeriodFrequency>
        </calculationPeriodDates>
        <paymentDates>
          <paymentFrequency>6M</paymentFrequency>
        </paymentDates>
        <calculationPeriodAmount>
          <notionalSchedule>
            <notionalStepSchedule>
              <initialValue>{:.2}</initialValue>
              <currency>{}</currency>
            </notionalStepSchedule>
          </notionalSchedule>
          <fixedRateSchedule>
            <initialValue>{:.6}</initialValue>
          </fixedRateSchedule>
        </calculationPeriodAmount>
      </swapStream>
      <swapStream id="floatingLeg">
        <receiverPartyReference href="{}"/>
        <calculationPeriodDates>
          <effectiveDate>{}</effectiveDate>
          <terminationDate>{}</terminationDate>
          <calculationPeriodFrequency>3M</calculationPeriodFrequency>
        </calculationPeriodDates>
        <paymentDates>
          <paymentFrequency>3M</paymentFrequency>
        </paymentDates>
        <calculationPeriodAmount>
          <notionalSchedule>
            <notionalStepSchedule>
              <initialValue>{:.2}</initialValue>
              <currency>{}</currency>
            </notionalStepSchedule>
          </notionalSchedule>
          <floatingRateCalculation>
            <floatingRateIndex>{}</floatingRateIndex>
            <indexTenor>3M</indexTenor>
          </floatingRateCalculation>
        </calculationPeriodAmount>
      </swapStream>
    </swap>
  </trade>
</FpML>"#,
            trade.trade_id,
            trade.trade_date,
            if pay_fixed { &trade.counterparty_id } else { "SELF" },
            trade.trade_date,
            trade.maturity_date,
            trade.notional,
            trade.currency,
            fixed_rate,
            if pay_fixed { "SELF" } else { &trade.counterparty_id },
            trade.trade_date,
            trade.maturity_date,
            trade.notional,
            trade.currency,
            float_index
        )
    }

    /// Generate CDS FpML
    fn generate_cds_fpml(
        trade: &TradeRecord,
        reference_entity: &str,
        spread_bps: f64,
        is_protection_buyer: bool,
    ) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<FpML xmlns="http://www.fpml.org/FpML-5/confirmation" version="5-12">
  <trade>
    <tradeHeader>
      <partyTradeIdentifier>
        <tradeId>{}</tradeId>
      </partyTradeIdentifier>
      <tradeDate>{}</tradeDate>
    </tradeHeader>
    <creditDefaultSwap>
      <generalTerms>
        <effectiveDate>{}</effectiveDate>
        <scheduledTerminationDate>{}</scheduledTerminationDate>
        <buyerPartyReference href="{}"/>
        <sellerPartyReference href="{}"/>
        <referenceInformation>
          <referenceEntity>
            <entityName>{}</entityName>
          </referenceEntity>
        </referenceInformation>
      </generalTerms>
      <feeLeg>
        <periodicPayment>
          <fixedAmountCalculation>
            <calculationAmount>
              <currency>{}</currency>
              <amount>{:.2}</amount>
            </calculationAmount>
            <fixedRate>{:.6}</fixedRate>
          </fixedAmountCalculation>
        </periodicPayment>
      </feeLeg>
      <protectionTerms>
        <calculationAmount>
          <currency>{}</currency>
          <amount>{:.2}</amount>
        </calculationAmount>
      </protectionTerms>
    </creditDefaultSwap>
  </trade>
</FpML>"#,
            trade.trade_id,
            trade.trade_date,
            trade.trade_date,
            trade.maturity_date,
            if is_protection_buyer { &trade.counterparty_id } else { "SELF" },
            if is_protection_buyer { "SELF" } else { &trade.counterparty_id },
            reference_entity,
            trade.currency,
            trade.notional,
            spread_bps / 10000.0,
            trade.currency,
            trade.notional
        )
    }

    /// Generate FX Forward FpML
    fn generate_fx_forward_fpml(
        trade: &TradeRecord,
        buy_currency: &str,
        sell_currency: &str,
        rate: f64,
    ) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<FpML xmlns="http://www.fpml.org/FpML-5/confirmation" version="5-12">
  <trade>
    <tradeHeader>
      <partyTradeIdentifier>
        <tradeId>{}</tradeId>
      </partyTradeIdentifier>
      <tradeDate>{}</tradeDate>
    </tradeHeader>
    <fxSingleLeg>
      <exchangedCurrency1>
        <payerPartyReference href="{}"/>
        <receiverPartyReference href="SELF"/>
        <paymentAmount>
          <currency>{}</currency>
          <amount>{:.2}</amount>
        </paymentAmount>
      </exchangedCurrency1>
      <exchangedCurrency2>
        <payerPartyReference href="SELF"/>
        <receiverPartyReference href="{}"/>
        <paymentAmount>
          <currency>{}</currency>
          <amount>{:.2}</amount>
        </paymentAmount>
      </exchangedCurrency2>
      <valueDate>{}</valueDate>
      <exchangeRate>
        <quotedCurrencyPair>
          <currency1>{}</currency1>
          <currency2>{}</currency2>
          <quoteBasis>Currency2PerCurrency1</quoteBasis>
        </quotedCurrencyPair>
        <rate>{:.6}</rate>
      </exchangeRate>
    </fxSingleLeg>
  </trade>
</FpML>"#,
            trade.trade_id,
            trade.trade_date,
            trade.counterparty_id,
            buy_currency,
            trade.notional,
            trade.counterparty_id,
            sell_currency,
            trade.notional * rate,
            trade.maturity_date,
            buy_currency,
            sell_currency,
            rate
        )
    }

    /// Generate Equity Option FpML
    fn generate_equity_option_fpml(
        trade: &TradeRecord,
        underlying: &str,
        strike: f64,
        is_call: bool,
    ) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<FpML xmlns="http://www.fpml.org/FpML-5/confirmation" version="5-12">
  <trade>
    <tradeHeader>
      <partyTradeIdentifier>
        <tradeId>{}</tradeId>
      </partyTradeIdentifier>
      <tradeDate>{}</tradeDate>
    </tradeHeader>
    <equityOption>
      <buyerPartyReference href="{}"/>
      <sellerPartyReference href="SELF"/>
      <optionType>{}</optionType>
      <underlyer>
        <singleUnderlyer>
          <equity>
            <instrumentId instrumentIdScheme="http://www.fpml.org/spec/2002/instrument-id-RIC">{}</instrumentId>
          </equity>
        </singleUnderlyer>
      </underlyer>
      <strike>
        <strikePrice>{:.4}</strikePrice>
      </strike>
      <numberOfOptions>{:.0}</numberOfOptions>
      <optionEntitlement>1</optionEntitlement>
      <equityExercise>
        <equityEuropeanExercise>
          <expirationDate>{}</expirationDate>
        </equityEuropeanExercise>
      </equityExercise>
      <premium>
        <payerPartyReference href="{}"/>
        <receiverPartyReference href="SELF"/>
        <paymentAmount>
          <currency>{}</currency>
          <amount>0</amount>
        </paymentAmount>
      </premium>
    </equityOption>
  </trade>
</FpML>"#,
            trade.trade_id,
            trade.trade_date,
            trade.counterparty_id,
            if is_call { "Call" } else { "Put" },
            underlying,
            strike,
            trade.notional / strike,
            trade.maturity_date,
            trade.counterparty_id,
            trade.currency
        )
    }

    /// Generate generic FpML for unsupported types
    fn generate_generic_fpml(trade: &TradeRecord) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<FpML xmlns="http://www.fpml.org/FpML-5/confirmation" version="5-12">
  <trade>
    <tradeHeader>
      <partyTradeIdentifier>
        <tradeId>{}</tradeId>
      </partyTradeIdentifier>
      <tradeDate>{}</tradeDate>
    </tradeHeader>
    <genericProduct>
      <productType>{:?}</productType>
      <notional>
        <currency>{}</currency>
        <amount>{:.2}</amount>
      </notional>
      <terminationDate>{}</terminationDate>
    </genericProduct>
  </trade>
</FpML>"#,
            trade.trade_id,
            trade.trade_date,
            trade.instrument_type,
            trade.currency,
            trade.notional,
            trade.maturity_date
        )
    }

    /// Generate multiple FpML documents
    pub fn to_fpml_batch(trades: &[TradeRecord]) -> Vec<String> {
        trades.iter().map(Self::to_fpml).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trade_source::{FrontOffice, TradeSource};

    #[test]
    fn test_fpml_irs_generation() {
        let fo = FrontOffice::new();
        let trades = fo.generate_irs_trades(1);
        let fpml = FpmlGenerator::to_fpml(&trades[0]);
        assert!(fpml.contains("<swap>"));
        assert!(fpml.contains(&trades[0].trade_id));
        assert!(fpml.contains("FpML"));
    }

    #[test]
    fn test_fpml_equity_option_generation() {
        let fo = FrontOffice::new();
        let trades = fo.generate_equity_options(1);
        let fpml = FpmlGenerator::to_fpml(&trades[0]);
        assert!(fpml.contains("<equityOption>"));
        assert!(fpml.contains(&trades[0].trade_id));
    }

    #[test]
    fn test_fpml_fx_forward_generation() {
        let fo = FrontOffice::new();
        let trades = fo.generate_fx_forwards(1);
        let fpml = FpmlGenerator::to_fpml(&trades[0]);
        assert!(fpml.contains("<fxSingleLeg>"));
        assert!(fpml.contains(&trades[0].trade_id));
    }

    #[test]
    fn test_fpml_cds_generation() {
        let fo = FrontOffice::new();
        let trades = fo.generate_cds_trades(1);
        let fpml = FpmlGenerator::to_fpml(&trades[0]);
        assert!(fpml.contains("<creditDefaultSwap>"));
        assert!(fpml.contains(&trades[0].trade_id));
    }

    #[test]
    fn test_fpml_fx_option_generation() {
        let fo = FrontOffice::new();
        let trades = fo.generate_fx_options(1);
        let fpml = FpmlGenerator::to_fpml(&trades[0]);
        assert!(fpml.contains("<fxOption>"));
        assert!(fpml.contains(&trades[0].trade_id));
    }

    #[test]
    fn test_fpml_equity_forward_generation() {
        let fo = FrontOffice::new();
        let trades = fo.generate_equity_forwards(1);
        let fpml = FpmlGenerator::to_fpml(&trades[0]);
        assert!(fpml.contains("<equityForward>"));
        assert!(fpml.contains(&trades[0].trade_id));
    }

    #[test]
    fn test_fpml_batch_generation() {
        let fo = FrontOffice::new();
        let trades = fo.generate_trades(10);
        let fpml_docs = FpmlGenerator::to_fpml_batch(&trades);
        assert_eq!(fpml_docs.len(), 10);

        // All docs should be valid XML-ish
        for doc in &fpml_docs {
            assert!(doc.starts_with("<?xml"));
            assert!(doc.contains("<FpML"));
            assert!(doc.contains("</FpML>"));
        }
    }
}
