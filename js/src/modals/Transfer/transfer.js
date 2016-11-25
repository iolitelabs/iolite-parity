// Copyright 2015, 2016 Ethcore (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

import BigNumber from 'bignumber.js';
import React, { Component, PropTypes } from 'react';
import { connect } from 'react-redux';
import { bindActionCreators } from 'redux';
import ActionDoneAll from 'material-ui/svg-icons/action/done-all';
import ContentClear from 'material-ui/svg-icons/content/clear';
import NavigationArrowBack from 'material-ui/svg-icons/navigation/arrow-back';
import NavigationArrowForward from 'material-ui/svg-icons/navigation/arrow-forward';

import { BusyStep, CompletedStep, Button, IdentityIcon, Modal, TxHash } from '../../ui';
import { DEFAULT_GAS, DEFAULT_GASPRICE, MAX_GAS_ESTIMATION } from '../../util/constants';

import Details from './Details';
import Extras from './Extras';
import ERRORS from './errors';
import styles from './transfer.css';

import { ERROR_CODES } from '../../api/transport/error';

const TITLES = {
  transfer: 'transfer details',
  sending: 'sending',
  complete: 'complete',
  extras: 'extra information',
  rejected: 'rejected'
};
const STAGES_BASIC = [TITLES.transfer, TITLES.sending, TITLES.complete];
const STAGES_EXTRA = [TITLES.transfer, TITLES.extras, TITLES.sending, TITLES.complete];

class Transfer extends Component {
  static contextTypes = {
    api: PropTypes.object.isRequired,
    store: PropTypes.object.isRequired
  }

  static propTypes = {
    account: PropTypes.object,
    balance: PropTypes.object,
    balances: PropTypes.object,
    gasLimit: PropTypes.object.isRequired,
    images: PropTypes.object.isRequired,
    onClose: PropTypes.func
  }

  state = {
    stage: 0,
    data: '',
    dataError: null,
    extras: false,
    gas: DEFAULT_GAS,
    gasEst: '0',
    gasError: null,
    gasLimitError: null,
    gasPrice: DEFAULT_GASPRICE,
    gasPriceHistogram: {},
    gasPriceError: null,
    recipient: '',
    recipientError: ERRORS.requireRecipient,
    sending: false,
    tag: 'ETH',
    total: '0.0',
    totalError: null,
    value: '0.0',
    valueAll: false,
    valueError: null,
    isEth: true,
    busyState: null,
    rejected: false
  }

  componentDidMount () {
    this.getDefaults();
  }

  render () {
    const { stage, extras, rejected } = this.state;

    const steps = [].concat(extras ? STAGES_EXTRA : STAGES_BASIC);

    if (rejected) {
      steps[steps.length - 1] = TITLES.rejected;
    }

    return (
      <Modal
        actions={ this.renderDialogActions() }
        current={ stage }
        steps={ steps }
        waiting={ extras ? [2] : [1] }
        visible
        scroll
      >
        { this.renderWarning() }
        { this.renderPage() }
      </Modal>
    );
  }

  renderAccount () {
    const { account } = this.props;

    return (
      <div className={ styles.hdraccount }>
        <div className={ styles.hdrimage }>
          <IdentityIcon
            inline center
            address={ account.address } />
        </div>
        <div className={ styles.hdrdetails }>
          <div className={ styles.hdrname }>
            { account.name || 'Unnamed' }
          </div>
          <div className={ styles.hdraddress }>
            { account.address }
          </div>
        </div>
      </div>
    );
  }

  renderPage () {
    const { extras, stage } = this.state;

    if (stage === 0) {
      return this.renderDetailsPage();
    } else if (stage === 1 && extras) {
      return this.renderExtrasPage();
    }

    return this.renderCompletePage();
  }

  renderCompletePage () {
    const { sending, txhash, busyState, rejected } = this.state;

    if (rejected) {
      return (
        <BusyStep
          title='The transaction has been rejected'
          state='You can safely close this window, the transfer will not occur.'
        />
      );
    }

    if (sending) {
      return (
        <BusyStep
          title='The transaction is in progress'
          state={ busyState } />
      );
    }

    return (
      <CompletedStep>
        <TxHash hash={ txhash } />
      </CompletedStep>
    );
  }

  renderDetailsPage () {
    const { account, balance, images } = this.props;

    return (
      <Details
        address={ account.address }
        all={ this.state.valueAll }
        balance={ balance }
        extras={ this.state.extras }
        images={ images }
        recipient={ this.state.recipient }
        recipientError={ this.state.recipientError }
        tag={ this.state.tag }
        total={ this.state.total }
        totalError={ this.state.totalError }
        value={ this.state.value }
        valueError={ this.state.valueError }
        onChange={ this.onUpdateDetails } />
    );
  }

  renderExtrasPage () {
    if (!this.state.gasPriceHistogram) {
      return null;
    }

    return (
      <Extras
        isEth={ this.state.isEth }
        data={ this.state.data }
        dataError={ this.state.dataError }
        gas={ this.state.gas }
        gasEst={ this.state.gasEst }
        gasError={ this.state.gasError }
        gasPrice={ this.state.gasPrice }
        gasPriceDefault={ this.state.gasPriceDefault }
        gasPriceError={ this.state.gasPriceError }
        gasPriceHistogram={ this.state.gasPriceHistogram }
        total={ this.state.total }
        totalError={ this.state.totalError }
        onChange={ this.onUpdateDetails } />
    );
  }

  renderDialogActions () {
    const { account } = this.props;
    const { extras, sending, stage } = this.state;

    const cancelBtn = (
      <Button
        icon={ <ContentClear /> }
        label='Cancel'
        onClick={ this.onClose } />
    );
    const nextBtn = (
      <Button
        disabled={ !this.isValid() }
        icon={ <NavigationArrowForward /> }
        label='Next'
        onClick={ this.onNext } />
    );
    const prevBtn = (
      <Button
        icon={ <NavigationArrowBack /> }
        label='Back'
        onClick={ this.onPrev } />
    );
    const sendBtn = (
      <Button
        disabled={ !this.isValid() || sending }
        icon={ <IdentityIcon address={ account.address } button /> }
        label='Send'
        onClick={ this.onSend } />
    );
    const doneBtn = (
      <Button
        icon={ <ActionDoneAll /> }
        label='Close'
        onClick={ this.onClose } />
    );

    switch (stage) {
      case 0:
        return extras
          ? [cancelBtn, nextBtn]
          : [cancelBtn, sendBtn];
      case 1:
        return extras
          ? [cancelBtn, prevBtn, sendBtn]
          : [doneBtn];
      default:
        return [doneBtn];
    }
  }

  renderWarning () {
    const { gasLimitError } = this.state;

    if (!gasLimitError) {
      return null;
    }

    return (
      <div className={ styles.warning }>
        { gasLimitError }
      </div>
    );
  }

  isValid () {
    const detailsValid = !this.state.recipientError && !this.state.valueError && !this.state.totalError;
    const extrasValid = !this.state.gasError && !this.state.gasPriceError && !this.state.totalError;
    const verifyValid = !this.state.passwordError;

    switch (this.state.stage) {
      case 0:
        return detailsValid;

      case 1:
        return this.state.extras ? extrasValid : verifyValid;

      case 2:
        return verifyValid;
    }
  }

  onNext = () => {
    this.setState({
      stage: this.state.stage + 1
    });
  }

  onPrev = () => {
    this.setState({
      stage: this.state.stage - 1
    });
  }

  _onUpdateAll (valueAll) {
    this.setState({
      valueAll
    }, this.recalculateGas);
  }

  _onUpdateExtras (extras) {
    this.setState({
      extras
    });
  }

  _onUpdateData (data) {
    this.setState({
      data
    }, this.recalculateGas);
  }

  validatePositiveNumber (num) {
    try {
      const v = new BigNumber(num);
      if (v.lt(0)) {
        return ERRORS.invalidAmount;
      }
    } catch (e) {
      return ERRORS.invalidAmount;
    }

    return null;
  }

  validateDecimals (num) {
    const { balance } = this.props;
    const { tag } = this.state;

    if (tag === 'ETH') {
      return null;
    }

    const token = balance.tokens.find((balance) => balance.token.tag === tag).token;
    const s = new BigNumber(num).mul(token.format || 1).toFixed();

    if (s.indexOf('.') !== -1) {
      return ERRORS.invalidDecimals;
    }

    return null;
  }

  _onUpdateGas (gas) {
    const gasError = this.validatePositiveNumber(gas);

    this.setState({
      gas,
      gasError
    }, this.recalculate);
  }

  _onUpdateGasPrice (gasPrice) {
    const gasPriceError = this.validatePositiveNumber(gasPrice);

    this.setState({
      gasPrice,
      gasPriceError
    }, this.recalculate);
  }

  _onUpdateRecipient (recipient) {
    const { api } = this.context;
    let recipientError = null;

    if (!recipient || !recipient.length) {
      recipientError = ERRORS.requireRecipient;
    } else if (!api.util.isAddressValid(recipient)) {
      recipientError = ERRORS.invalidAddress;
    }

    this.setState({
      recipient,
      recipientError
    }, this.recalculateGas);
  }

  _onUpdateTag (tag) {
    const { balance } = this.props;

    this.setState({
      tag,
      isEth: tag === balance.tokens[0].token.tag
    }, this.recalculateGas);
  }

  _onUpdateValue (value) {
    let valueError = this.validatePositiveNumber(value);

    if (!valueError) {
      valueError = this.validateDecimals(value);
    }

    this.setState({
      value,
      valueError
    }, this.recalculateGas);
  }

  onUpdateDetails = (type, value) => {
    switch (type) {
      case 'all':
        return this._onUpdateAll(value);

      case 'extras':
        return this._onUpdateExtras(value);

      case 'data':
        return this._onUpdateData(value);

      case 'gas':
        return this._onUpdateGas(value);

      case 'gasPrice':
        return this._onUpdateGasPrice(value);

      case 'recipient':
        return this._onUpdateRecipient(value);

      case 'tag':
        return this._onUpdateTag(value);

      case 'value':
        return this._onUpdateValue(value);
    }
  }

  _sendEth () {
    const { api } = this.context;
    const { account } = this.props;
    const { data, gas, gasPrice, recipient, value } = this.state;

    const options = {
      from: account.address,
      to: recipient,
      gas,
      gasPrice,
      value: api.util.toWei(value || 0)
    };

    if (data && data.length) {
      options.data = data;
    }

    return api.parity.postTransaction(options);
  }

  _sendToken () {
    const { account, balance } = this.props;
    const { gas, gasPrice, recipient, value, tag } = this.state;
    const token = balance.tokens.find((balance) => balance.token.tag === tag).token;

    return token.contract.instance.transfer
      .postTransaction({
        from: account.address,
        to: token.address,
        gas,
        gasPrice
      }, [
        recipient,
        new BigNumber(value).mul(token.format).toFixed(0)
      ]);
  }

  onSend = () => {
    const { api } = this.context;

    this.onNext();

    this.setState({ sending: true }, () => {
      (this.state.isEth
        ? this._sendEth()
        : this._sendToken()
      ).then((requestId) => {
        this.setState({ busyState: 'Waiting for authorization in the Parity Signer' });

        return api
          .pollMethod('parity_checkRequest', requestId)
          .catch((e) => {
            if (e.code === ERROR_CODES.REQUEST_REJECTED) {
              this.setState({ rejected: true });
              return false;
            }

            throw e;
          });
      })
      .then((txhash) => {
        this.onNext();
        this.setState({
          sending: false,
          txhash,
          busyState: 'Your transaction has been posted to the network'
        });
      })
      .catch((error) => {
        console.log('send', error);

        this.setState({
          sending: false
        });

        this.newError(error);
      });
    });
  }

  onClose = () => {
    this.setState({ stage: 0 }, () => {
      this.props.onClose && this.props.onClose();
    });
  }

  _estimateGasToken () {
    const { account, balance } = this.props;
    const { recipient, value, tag } = this.state;
    const token = balance.tokens.find((balance) => balance.token.tag === tag).token;

    return token.contract.instance.transfer
      .estimateGas({
        gas: MAX_GAS_ESTIMATION,
        from: account.address,
        to: token.address
      }, [
        recipient,
        new BigNumber(value || 0).mul(token.format).toFixed(0)
      ]);
  }

  _estimateGasEth () {
    const { api } = this.context;
    const { account } = this.props;
    const { data, recipient, value } = this.state;
    const options = {
      gas: MAX_GAS_ESTIMATION,
      from: account.address,
      to: recipient,
      value: api.util.toWei(value || 0)
    };

    if (data && data.length) {
      options.data = data;
    }

    return api.eth.estimateGas(options);
  }

  recalculateGas = () => {
    if (!this.isValid()) {
      this.setState({
        gas: '0'
      }, this.recalculate);
      return;
    }

    const { gasLimit } = this.props;

    (this.state.isEth
      ? this._estimateGasEth()
      : this._estimateGasToken()
    ).then((gasEst) => {
      let gas = gasEst;
      let gasLimitError = null;

      if (gas.gt(DEFAULT_GAS)) {
        gas = gas.mul(1.2);
      }

      if (gas.gte(MAX_GAS_ESTIMATION)) {
        gasLimitError = ERRORS.gasException;
      } else if (gas.gt(gasLimit)) {
        gasLimitError = ERRORS.gasBlockLimit;
      }

      this.setState({
        gas: gas.toFixed(0),
        gasEst: gasEst.toFormat(),
        gasLimitError
      }, this.recalculate);
    })
    .catch((error) => {
      console.error('etimateGas', error);
      this.recalculate();
    });
  }

  recalculate = () => {
    const { api } = this.context;
    const { account, balance } = this.props;

    if (!account || !balance) {
      return;
    }

    const { gas, gasPrice, tag, valueAll, isEth } = this.state;
    const gasTotal = new BigNumber(gasPrice || 0).mul(new BigNumber(gas || 0));
    const balance_ = balance.tokens.find((b) => tag === b.token.tag);
    const availableEth = new BigNumber(balance.tokens[0].value);
    const available = new BigNumber(balance_.value);
    const format = new BigNumber(balance_.token.format || 1);

    let { value, valueError } = this.state;
    let totalEth = gasTotal;
    let totalError = null;

    if (valueAll) {
      if (isEth) {
        const bn = api.util.fromWei(availableEth.minus(gasTotal));
        value = (bn.lt(0) ? new BigNumber(0.0) : bn).toString();
      } else {
        value = available.div(format).toString();
      }
    }

    if (isEth) {
      totalEth = totalEth.plus(api.util.toWei(value || 0));
    }

    if (new BigNumber(value || 0).gt(available.div(format))) {
      valueError = ERRORS.largeAmount;
    } else if (valueError === ERRORS.largeAmount) {
      valueError = null;
    }

    if (totalEth.gt(availableEth)) {
      totalError = ERRORS.largeAmount;
    }

    this.setState({
      total: api.util.fromWei(totalEth).toString(),
      totalError,
      value,
      valueError
    });
  }

  getDefaults = () => {
    const { api } = this.context;

    Promise
      .all([
        api.parity.gasPriceHistogram(),
        api.eth.gasPrice()
      ])
      .then(([gasPriceHistogram, gasPrice]) => {
        this.setState({
          gasPrice: gasPrice.toString(),
          gasPriceDefault: gasPrice.toFormat(),
          gasPriceHistogram
        }, this.recalculate);
      })
      .catch((error) => {
        console.warn('getDefaults', error);
      });
  }

  newError = (error) => {
    const { store } = this.context;

    store.dispatch({ type: 'newError', error });
  }
}

function mapStateToProps (state) {
  const { gasLimit } = state.nodeStatus;

  return { gasLimit };
}

function mapDispatchToProps (dispatch) {
  return bindActionCreators({}, dispatch);
}

export default connect(
  mapStateToProps,
  mapDispatchToProps
)(Transfer);
